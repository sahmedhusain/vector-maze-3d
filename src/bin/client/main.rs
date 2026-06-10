use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use macroquad::prelude::*;

use mp::{
    ClientMessage, LaserEffect, PlayerAction, ServerMessage,
    dir_idx_to_angle, MatchState,
};

pub mod launcher;
pub mod render;
pub mod minimap;
pub mod editor;

use launcher::update_and_draw_launcher;
use render::render_3d_viewport;
use minimap::render_minimap;
use editor::run_editor_tick;

pub const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);

pub struct VisualPlayer {
    pub id: u32,
    pub name: String,
    pub vx: f32,
    pub vy: f32,
    pub vtheta: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_theta: f32,
    pub score: i32,
    pub deaths: i32,
    pub health: i32,
    pub is_alive: bool,
    pub is_bot: bool,
}

enum ClientState {
    Launcher,
    Joining,
    Playing,
    Rejected { reason: String },
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars 3D".to_string(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        fullscreen: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font_bytes = include_bytes!("../../../Roboto-Regular.ttf");
    let font = load_ttf_font_from_bytes(font_bytes).expect("Failed to load Roboto font");

    let args: Vec<String> = std::env::args().collect();
    let mut cli_ip = None;
    let mut cli_user = None;

    let mut i = 1;
    let mut positional_args = Vec::new();
    while i < args.len() {
        match args[i].as_str() {
            "--ip" | "-i" => {
                if i + 1 < args.len() {
                    cli_ip = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--name" | "-n" | "--username" => {
                if i + 1 < args.len() {
                    cli_user = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Maze Wars Client Usage:");
                println!("  --ip, -i <ip>              Server IP:port to connect to");
                println!("  --name, -n <username>      Player username");
                println!("  --help, -h                 Show this help message");
                std::process::exit(0);
            }
            arg => {
                if !arg.starts_with('-') {
                    positional_args.push(arg.to_string());
                }
                i += 1;
            }
        }
    }

    if cli_ip.is_none() && !positional_args.is_empty() {
        cli_ip = Some(positional_args[0].clone());
    }
    if cli_user.is_none() && positional_args.len() > 1 {
        cli_user = Some(positional_args[1].clone());
    }

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");
    socket.set_nonblocking(true).expect("Failed to set non-blocking");

    let server_tick_state = Arc::new(Mutex::new(None));
    let welcome_map_state = Arc::new(Mutex::new(None));
    let reject_state = Arc::new(Mutex::new(None));

    let socket_clone = socket.try_clone().unwrap();
    let state_recv = server_tick_state.clone();
    let map_recv = welcome_map_state.clone();
    let reject_recv = reject_state.clone();

    // Net thread
    std::thread::spawn(move || {
        let mut buf = [0; 65536];
        loop {
            match socket_clone.recv_from(&mut buf) {
                Ok((size, _)) => {
                    if let Ok(msg) = bincode::deserialize::<ServerMessage>(&buf[..size]) {
                        match msg {
                            ServerMessage::Welcome {
                                player_id,
                                map_width,
                                map_height,
                                map_cells,
                                level_index,
                                is_host: _,
                            } => {
                                let mut lock = map_recv.lock().unwrap();
                                *lock = Some((player_id, map_width, map_height, map_cells, level_index));
                            }
                            ServerMessage::Tick(tick) => {
                                let mut lock = state_recv.lock().unwrap();
                                *lock = Some(tick);
                            }
                            ServerMessage::MapUpdate {
                                map_width,
                                map_height,
                                map_cells,
                                level_index,
                            } => {
                                let mut lock = map_recv.lock().unwrap();
                                let current_player_id = lock.as_ref().map(|x| x.0).unwrap_or(0);
                                *lock = Some((current_player_id, map_width, map_height, map_cells, level_index));
                            }
                            ServerMessage::Reject { reason } => {
                                let mut lock = reject_recv.lock().unwrap();
                                *lock = Some(reason);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        }
    });

    let mut client_state = ClientState::Launcher;
    let mut server_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let mut username = String::new();
    let mut launcher_state = launcher::LauncherState::new();

    if let (Some(ip), Some(user)) = (cli_ip, cli_user) {
        let mut final_ip = ip;
        if !final_ip.contains(':') {
            final_ip = format!("{}:{}", final_ip, mp::DEFAULT_PORT);
        }
        if let Ok(addr) = final_ip.parse::<SocketAddr>() {
            server_addr = addr;
            username = user;
            client_state = ClientState::Joining;
            println!("Joining game at {} as {}...", server_addr, username);
        } else {
            eprintln!("Error: Invalid server IP format!");
            std::process::exit(1);
        }
    }
    let mut my_player_id: u32 = 0;
    let mut map_width = 0;
    let mut map_height = 0;
    let mut map_cells = Vec::new();
    let mut current_level_idx = 0;

    let mut visual_players: std::collections::HashMap<u32, VisualPlayer> = std::collections::HashMap::new();
    let mut active_lasers: Vec<LaserEffect> = Vec::new();

    let mut last_join_sent = Instant::now() - Duration::from_secs(5);
    let mut last_heartbeat_sent = Instant::now();

    let mut edit_mode = false;
    let mut local_edit_cells = Vec::new();

    let mut current_match_state = MatchState::Lobby;
    let mut server_host_id: u32 = 0;
    let mut bots_enabled = true;

    loop {
        let screen_w = screen_width();
        let screen_h = screen_height();

        let view_x = 20.0f32;
        let view_y = 60.0f32;
        let sidebar_w = (screen_w * 0.28).clamp(240.0, 360.0);
        let sidebar_x = screen_w - sidebar_w - 20.0;
        let view_w = sidebar_x - 40.0;
        let view_h = screen_h - view_y - 60.0;

        if is_key_pressed(KeyCode::Escape) {
            let leave_pkt = ClientMessage::Leave;
            if let Ok(ser) = bincode::serialize(&leave_pkt) {
                let _ = socket.send_to(&ser, server_addr);
            }
            break;
        }

        let dt = get_frame_time();

        // 1. Connection check
        match &mut client_state {
            ClientState::Launcher => {
                if let Some((selected_ip, chosen_user)) = update_and_draw_launcher(&mut launcher_state, dt, &font) {
                    server_addr = selected_ip.parse().unwrap();
                    username = chosen_user;
                    client_state = ClientState::Joining;
                    last_join_sent = Instant::now() - Duration::from_secs(5);
                }
                next_frame().await;
                continue;
            }
            ClientState::Rejected { reason } => {
                clear_background(Color::new(0.12, 0.04, 0.04, 1.0));
                draw_text_centered("CONNECTION REJECTED", screen_w / 2.0, screen_h / 2.0 - 30.0, 32.0, RED, &font);
                draw_text_centered(reason, screen_w / 2.0, screen_h / 2.0 + 10.0, 20.0, WHITE, &font);
                draw_text_centered("Press ESC to exit", screen_w / 2.0, screen_h / 2.0 + 50.0, 14.0, GRAY, &font);
                next_frame().await;
                continue;
            }
            ClientState::Joining => {
                if last_join_sent.elapsed() > Duration::from_millis(500) {
                    let join_pkt = ClientMessage::Join { name: username.clone() };
                    if let Ok(ser) = bincode::serialize(&join_pkt) {
                        let _ = socket.send_to(&ser, server_addr);
                    }
                    last_join_sent = Instant::now();
                }

                // Check reject
                let reject_reason = {
                    let mut lock = reject_state.lock().unwrap();
                    lock.take()
                };
                if let Some(reason) = reject_reason {
                    client_state = ClientState::Rejected { reason };
                    continue;
                }

                // Check welcome
                let map_updated = {
                    let mut lock = welcome_map_state.lock().unwrap();
                    lock.take()
                };
                if let Some((p_id, w, h, cells, lvl_idx)) = map_updated {
                    my_player_id = p_id;
                    map_width = w;
                    map_height = h;
                    map_cells = cells.clone();
                    current_level_idx = lvl_idx;
                    local_edit_cells = cells;
                    client_state = ClientState::Playing;
                    println!("Joined!");
                }

                clear_background(Color::new(0.02, 0.04, 0.08, 1.0));
                draw_text_centered("CONNECTING TO SERVER...", screen_w / 2.0, screen_h / 2.0 - 20.0, 30.0, CYAN, &font);
                draw_text_centered(&format!("Server: {}", server_addr), screen_w / 2.0, screen_h / 2.0 + 20.0, 20.0, WHITE, &font);
                next_frame().await;
                continue;
            }
            ClientState::Playing => {
                if last_heartbeat_sent.elapsed() > Duration::from_secs(1) {
                    let hb = ClientMessage::Heartbeat;
                    if let Ok(ser) = bincode::serialize(&hb) {
                        let _ = socket.send_to(&ser, server_addr);
                    }
                    last_heartbeat_sent = Instant::now();
                }

                let map_updated = {
                    let mut lock = welcome_map_state.lock().unwrap();
                    lock.take()
                };
                if let Some((_, w, h, cells, lvl_idx)) = map_updated {
                    map_width = w;
                    map_height = h;
                    map_cells = cells.clone();
                    current_level_idx = lvl_idx;
                    if !edit_mode {
                        local_edit_cells = cells;
                    }
                }
            }
        }

        // 2. Process Tick
        let latest_tick = {
            let mut lock = server_tick_state.lock().unwrap();
            lock.take()
        };
        if let Some(tick) = latest_tick {
            active_lasers = tick.lasers;
            current_match_state = tick.match_state;
            server_host_id = tick.host_id;
            bots_enabled = tick.bots_enabled;

            let mut active_ids = std::collections::HashSet::new();
            for p_state in tick.players {
                active_ids.insert(p_state.id);
                let target_angle = dir_idx_to_angle(p_state.dir_idx);

                if let Some(v_player) = visual_players.get_mut(&p_state.id) {
                    v_player.target_x = p_state.x;
                    v_player.target_y = p_state.y;
                    v_player.target_theta = target_angle;
                    v_player.score = p_state.score;
                    v_player.deaths = p_state.deaths;
                    v_player.health = p_state.health;
                    v_player.is_alive = p_state.is_alive;
                    v_player.is_bot = p_state.is_bot;
                } else {
                    visual_players.insert(
                        p_state.id,
                        VisualPlayer {
                            id: p_state.id,
                            name: p_state.name.clone(),
                            vx: p_state.x,
                            vy: p_state.y,
                            vtheta: target_angle,
                            target_x: p_state.x,
                            target_y: p_state.y,
                            target_theta: target_angle,
                            score: p_state.score,
                            deaths: p_state.deaths,
                            health: p_state.health,
                            is_alive: p_state.is_alive,
                            is_bot: p_state.is_bot,
                        },
                    );
                }
            }
            visual_players.retain(|id, _| active_ids.contains(id));
        }

        for p in visual_players.values_mut() {
            p.vx += (p.target_x - p.vx) * dt * 12.0;
            p.vy += (p.target_y - p.vy) * dt * 12.0;

            let diff = p.target_theta - p.vtheta;
            let diff = (diff + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI) - std::f32::consts::PI;
            p.vtheta += diff * dt * 12.0;
        }

        let local_player_opt = visual_players.get(&my_player_id);
        let is_host = my_player_id == server_host_id;

        // 3. Inputs
        if !edit_mode {
            if current_match_state == MatchState::Playing {
                let is_alive = local_player_opt.map(|p| p.is_alive).unwrap_or(false);
                if is_alive {
                    let mut action = None;
                    if is_key_pressed(KeyCode::W) || is_key_pressed(KeyCode::Up) {
                        action = Some(PlayerAction::MoveForward);
                    } else if is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down) {
                        action = Some(PlayerAction::MoveBackward);
                    } else if is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::Left) {
                        action = Some(PlayerAction::TurnLeft);
                    } else if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Right) {
                        action = Some(PlayerAction::TurnRight);
                    }

                    if let Some(act) = action {
                        let inp = ClientMessage::Input { action: act };
                        if let Ok(ser) = bincode::serialize(&inp) {
                            let _ = socket.send_to(&ser, server_addr);
                        }
                    }

                    if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
                        let (mx, my) = mouse_position();
                        let click_in_viewport = mx >= view_x && mx <= view_x + view_w && my >= view_y && my <= view_y + view_h;
                        if is_key_pressed(KeyCode::Space) || click_in_viewport {
                            let shoot = ClientMessage::Shoot;
                            if let Ok(ser) = bincode::serialize(&shoot) {
                                  let _ = socket.send_to(&ser, server_addr);
                            }
                        }
                    }
                }
            } else if current_match_state == MatchState::Lobby && is_host {
                // Host lobby inputs
                if is_key_pressed(KeyCode::G) {
                    let start = ClientMessage::StartGame;
                    if let Ok(ser) = bincode::serialize(&start) {
                        let _ = socket.send_to(&ser, server_addr);
                    }
                } else if is_key_pressed(KeyCode::B) {
                    let toggle = ClientMessage::ToggleBots;
                    if let Ok(ser) = bincode::serialize(&toggle) {
                        let _ = socket.send_to(&ser, server_addr);
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::E) && is_host {
            edit_mode = !edit_mode;
            if edit_mode {
                local_edit_cells = map_cells.clone();
            }
        }

        if !edit_mode && is_host {
            let mut requested_level = None;
            if is_key_pressed(KeyCode::Key1) {
                requested_level = Some(0);
            } else if is_key_pressed(KeyCode::Key2) {
                requested_level = Some(1);
            } else if is_key_pressed(KeyCode::Key3) {
                requested_level = Some(2);
            } else if is_key_pressed(KeyCode::Key4) {
                requested_level = Some(3);
            }

            if let Some(lvl_idx) = requested_level {
                let req = ClientMessage::RequestLevel { level_idx: lvl_idx };
                if let Ok(ser) = bincode::serialize(&req) {
                    let _ = socket.send_to(&ser, server_addr);
                }
            }
        }

        // 4. Render UI
        clear_background(Color::new(0.01, 0.02, 0.04, 1.0));

        draw_text_ex_font("MAZE WARS 3D", 20.0, 35.0, 28.0, CYAN, &font);
        let status_str = if edit_mode {
            "EDITOR MODE"
        } else if current_match_state == MatchState::Lobby {
            "MATCH LOBBY"
        } else {
            "PLAY MODE"
        };
        draw_text_ex_font(&format!("MODE: {}", status_str), sidebar_x, 35.0, 20.0, YELLOW, &font);

        draw_rectangle_lines(view_x - 1.0, view_y - 1.0, view_w + 2.0, view_h + 2.0, 2.0, Color::new(0.1, 0.2, 0.3, 1.0));

        if edit_mode {
            run_editor_tick(
                view_x,
                view_y,
                view_w,
                view_h,
                &socket,
                server_addr,
                &mut local_edit_cells,
                &mut map_width,
                &mut map_height,
                &font,
            );
        } else if current_match_state == MatchState::Lobby {
            // Draw Lobby screen
            draw_rectangle(view_x, view_y, view_w, view_h, Color::new(0.04, 0.06, 0.12, 1.0));
            draw_text_centered("MATCH LOBBY", view_x + view_w / 2.0, view_y + 80.0, 32.0, CYAN, &font);
            
            draw_text_centered("Players in Lobby (Max 4):", view_x + view_w / 2.0, view_y + 140.0, 18.0, LIGHTGRAY, &font);
            
            let mut ly = view_y + 180.0;
            let mut humans = visual_players.values().filter(|p| !p.is_bot).collect::<Vec<&VisualPlayer>>();
            humans.sort_by_key(|p| p.id);
            for p in humans {
                let role = if p.id == server_host_id { " [HOST]" } else { "" };
                let p_color = if p.id == my_player_id { GREEN } else { WHITE };
                draw_text_centered(&format!("- {}{}", p.name, role), view_x + view_w / 2.0, ly, 16.0, p_color, &font);
                ly += 25.0;
            }

            let bot_status = if bots_enabled { "ENABLED" } else { "DISABLED" };
            let bot_color = if bots_enabled { GREEN } else { RED };
            draw_text_centered(&format!("Bots AI Option: {}", bot_status), view_x + view_w / 2.0, view_y + 320.0, 16.0, bot_color, &font);

            if is_host {
                draw_text_centered("Press [ G ] to Start Match", view_x + view_w / 2.0, view_y + 400.0, 18.0, YELLOW, &font);
                draw_text_centered("Press [ B ] to Toggle AI Bots", view_x + view_w / 2.0, view_y + 430.0, 14.0, GRAY, &font);
            } else {
                draw_text_centered("Waiting for Host to start...", view_x + view_w / 2.0, view_y + 400.0, 16.0, LIGHTGRAY, &font);
            }
        } else {
            // Render Viewport
            if let Some(local_player) = local_player_opt {
                if local_player.is_alive {
                    render_3d_viewport(view_x, view_y, view_w, view_h, local_player, &visual_players, &map_cells, map_width, map_height, &active_lasers, &font);
                } else {
                    draw_rectangle(view_x, view_y, view_w, view_h, Color::new(0.15, 0.02, 0.02, 0.8));
                    draw_text_centered("YOU ARE ELIMINATED", view_x + view_w / 2.0, view_y + view_h / 2.0 - 20.0, 32.0, RED, &font);
                    draw_text_centered("RESPAWNING SOON...", view_x + view_w / 2.0, view_y + view_h / 2.0 + 25.0, 20.0, WHITE, &font);
                }
            } else {
                draw_rectangle(view_x, view_y, view_w, view_h, BLACK);
                draw_text_centered("SPAWNING...", view_x + view_w / 2.0, view_y + view_h / 2.0, 24.0, WHITE, &font);
            }
        }

        let fps = get_fps();
        let fps_color = if fps >= 60 { GREEN } else if fps >= 50 { YELLOW } else { RED };
        draw_text_ex_font(&format!("FPS: {}", fps), sidebar_x + 10.0, 32.0, 18.0, fps_color, &font);

        // Scoreboard Panel
        draw_rectangle(sidebar_x, view_y, sidebar_w, 200.0, Color::new(0.04, 0.06, 0.12, 0.8));
        draw_rectangle_lines(sidebar_x, view_y, sidebar_w, 200.0, 1.0, Color::new(0.1, 0.15, 0.25, 1.0));
        draw_text_ex_font("SCOREBOARD", sidebar_x + 10.0, view_y + 20.0, 18.0, CYAN, &font);

        draw_text_ex_font("Player", sidebar_x + 10.0, view_y + 45.0, 13.0, GRAY, &font);
        draw_text_ex_font("K", sidebar_x + 170.0, view_y + 45.0, 13.0, GRAY, &font);
        draw_text_ex_font("D", sidebar_x + 200.0, view_y + 45.0, 13.0, GRAY, &font);
        draw_text_ex_font("HP", sidebar_x + 230.0, view_y + 45.0, 13.0, GRAY, &font);

        let mut sorted_players: Vec<&VisualPlayer> = visual_players.values().collect();
        sorted_players.sort_by(|a, b| {
            b.score.cmp(&a.score).then_with(|| a.deaths.cmp(&b.deaths))
        });

        let mut row_y = view_y + 65.0;
        for p in sorted_players.iter().take(8) {
            let name_color = if p.id == my_player_id { GREEN } else if p.is_bot { ORANGE } else { WHITE };
            let name_display = if p.name.len() > 14 { format!("{}...", &p.name[..11]) } else { p.name.clone() };

            draw_text_ex_font(&name_display, sidebar_x + 10.0, row_y, 14.0, name_color, &font);
            draw_text_ex_font(&p.score.to_string(), sidebar_x + 170.0, row_y, 14.0, name_color, &font);
            draw_text_ex_font(&p.deaths.to_string(), sidebar_x + 200.0, row_y, 14.0, name_color, &font);
            
            let hp_color = if p.health > 50 { GREEN } else if p.health > 25 { YELLOW } else { RED };
            let hp_text = if p.is_alive { p.health.to_string() } else { "DEAD".to_string() };
            draw_text_ex_font(&hp_text, sidebar_x + 230.0, row_y, 14.0, hp_color, &font);

            row_y += 18.0;
        }

        // Mini-Map Panel
        let map_area_y = view_y + 220.0;
        let map_area_h = (screen_h - map_area_y - 150.0).clamp(160.0, 320.0);
        draw_rectangle(sidebar_x, map_area_y, sidebar_w, map_area_h, Color::new(0.04, 0.06, 0.12, 0.8));
        draw_rectangle_lines(sidebar_x, map_area_y, sidebar_w, map_area_h, 1.0, Color::new(0.1, 0.15, 0.25, 1.0));
        draw_text_ex_font("MINI MAP", sidebar_x + 10.0, map_area_y + 20.0, 18.0, CYAN, &font);

        if map_width > 0 && map_height > 0 {
            render_minimap(
                sidebar_x + (sidebar_w - 240.0) / 2.0,
                map_area_y + 35.0,
                240.0,
                map_area_h - 50.0,
                map_width,
                map_height,
                &map_cells,
                my_player_id,
                &visual_players,
                &active_lasers,
            );
        }

        // Keyboard Instructions Panel
        let help_y = map_area_y + map_area_h + 15.0;
        let help_h = (screen_h - help_y - 20.0).max(95.0);
        draw_rectangle(sidebar_x, help_y, sidebar_w, help_h, Color::new(0.04, 0.06, 0.12, 0.8));
        draw_rectangle_lines(sidebar_x, help_y, sidebar_w, help_h, 1.0, Color::new(0.1, 0.15, 0.25, 1.0));
        draw_text_ex_font("KEYBOARD HELP", sidebar_x + 10.0, help_y + 20.0, 14.0, CYAN, &font);

        let help_text = if edit_mode {
            vec![
                "L-Click: Place Wall  | R-Click: Erase Wall",
                "R: Random Maze       | C: Clear Grid Map",
                "U: Upload Custom Map to Server",
                "E: Exit Editor Mode",
            ]
        } else if current_match_state == MatchState::Lobby {
            if is_host {
                vec![
                    "G: Start Game Match",
                    "B: Toggle Bots AI",
                    "1, 2, 3: Choose Level map | 4: Random",
                    "E: Open Level Editor",
                ]
            } else {
                vec![
                    "Waiting for host to start...",
                    "",
                    "",
                    "",
                ]
            }
        } else {
            vec![
                "WASD / Arrow Keys: Move & Turn",
                "Space / Mouse Click: Shoot Laser",
                "ESC: Return to Lobby",
                "",
            ]
        };

        let mut hy = help_y + 40.0;
        for line in help_text {
            draw_text_ex_font(line, sidebar_x + 10.0, hy, 11.0, LIGHTGRAY, &font);
            hy += 16.0;
        }

        let level_name = match current_level_idx {
            0 => "Level 1 (Simple)".to_string(),
            1 => "Level 2 (Medium)".to_string(),
            2 => "Level 3 (Labyrinth)".to_string(),
            3 => "Randomly Generated Labyrinth".to_string(),
            4 => "Custom Client Map".to_string(),
            _ => "Unknown Labyrinth".to_string(),
        };
        draw_text_ex_font(&format!("Level: {}", level_name), view_x, view_y + view_h + 28.0, 16.0, LIGHTGRAY, &font);
        draw_text_ex_font("ESC: Leave Game", sidebar_x, view_y + view_h + 28.0, 16.0, RED, &font);

        next_frame().await;
    }
}

pub fn draw_text_centered(text: &str, cx: f32, cy: f32, size: f32, color: Color, font: &Font) {
    let text_w = measure_text(text, Some(font), size as u16, 1.0).width;
    draw_text_ex(
        text,
        cx - text_w / 2.0,
        cy,
        TextParams {
            font: Some(font),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

pub fn draw_text_ex_font(text: &str, x: f32, y: f32, size: f32, color: Color, font: &Font) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}
