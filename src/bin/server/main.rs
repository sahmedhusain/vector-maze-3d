use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use mp::*;

mod ai;
mod physics;

use ai::update_bots;
use physics::{cell_occupied_by_other, occupied_cells, reset_all_positions, find_random_spawn, fire_laser};

pub struct ClientSession {
    pub addr: SocketAddr,
    pub name: String,
    pub player_id: u32,
    pub last_packet_time: Instant,
    pub shoot_cooldown: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut port = DEFAULT_PORT;
    let mut bots_enabled = true;
    let mut current_level_idx = 0;

    let mut i = 1;
    let mut positional_args = Vec::new();
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse::<u16>() {
                        port = p;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--bots" | "-b" => {
                if i + 1 < args.len() {
                    match args[i + 1].to_lowercase().as_str() {
                        "true" | "yes" | "1" => bots_enabled = true,
                        "false" | "no" | "0" => bots_enabled = false,
                        _ => {}
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--level" | "-l" => {
                if i + 1 < args.len() {
                    if let Ok(l) = args[i + 1].parse::<usize>() {
                        if l <= 3 {
                            current_level_idx = l;
                        }
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Maze Wars Server Usage:");
                println!("  --port, -p <port>        Port to bind (default: 10500)");
                println!("  --bots, -b <true/false>  Enable/disable AI bots by default (default: true)");
                println!("  --level, -l <level_idx>  Initial level index 0-3 (default: 0)");
                println!("  --help, -h               Show this help message");
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

    if !positional_args.is_empty() {
        if let Ok(p) = positional_args[0].parse::<u16>() {
            port = p;
        }
    }

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind");
    socket.set_nonblocking(true).expect("Failed to set non-blocking");

    println!("MAZE WARS SERVER STARTED ON PORT {}", port);

    let mut level = mp::get_static_level(current_level_idx);

    let mut sessions: HashMap<SocketAddr, ClientSession> = HashMap::new();
    let mut players: HashMap<u32, PlayerState> = HashMap::new();
    let mut lasers: Vec<LaserEffect> = Vec::new();
    let mut next_player_id = 1;
    let mut tick_id: u64 = 0;
    let mut round_start_time: Option<Instant> = None;

    let mut match_state = MatchState::Lobby;
    let mut host_id: u32 = 0;
    let bot_ids: Vec<u32> = (1..=4).map(|i| 1000 + i).collect();

    let tick_duration = Duration::from_millis(1000 / TICK_RATE_HZ);
    let mut bot_cooldowns: HashMap<u32, u32> = HashMap::new();

    loop {
        let now = Instant::now();

        if match_state == MatchState::Playing {
            if let Some(start_time) = round_start_time {
                if now.duration_since(start_time).as_secs_f32() >= MATCH_DURATION_SECS {
                    match_state = MatchState::GameOver;
                    round_start_time = None;
                    lasers.clear();
                    println!("Round timer ended. Match over.");
                }
            }
        }

        // 1. Process client packets
        let mut buf = [0; 4096];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    if let Ok(msg) = bincode::deserialize::<ClientMessage>(&buf[..size]) {
                        handle_client_message(
                            &msg,
                            src_addr,
                            &socket,
                            &mut sessions,
                            &mut players,
                            &mut next_player_id,
                            &mut level,
                            &mut current_level_idx,
                            &mut lasers,
                            &mut match_state,
                            &mut round_start_time,
                            &mut host_id,
                            &mut bots_enabled,
                        );
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // 2. Timeout check
        let mut timed_out = Vec::new();
        for (addr, session) in sessions.iter() {
            if now.duration_since(session.last_packet_time).as_secs_f32() > HEARTBEAT_TIMEOUT_SECS {
                timed_out.push(*addr);
            }
        }
        for addr in timed_out {
            if let Some(session) = sessions.remove(&addr) {
                println!("Client {} timed out", session.name);
                players.remove(&session.player_id);

                // Reassign host if host timed out
                if session.player_id == host_id {
                    host_id = players.values().filter(|p| !p.is_bot).map(|p| p.id).next().unwrap_or(0);
                    println!("Host timed out. New Host: {}", host_id);
                }

                // If in-game and bots enabled, spawn a replacement bot
                if match_state == MatchState::Playing && bots_enabled {
                    let human_count = players.values().filter(|p| !p.is_bot).count();
                    if human_count > 0 {
                        spawn_replacement_bot(&mut players, &level);
                    }
                }
            }
        }

        // 3. Reset server if empty
        let human_count = players.values().filter(|p| !p.is_bot).count();
        if human_count == 0 {
            match_state = MatchState::Lobby;
            host_id = 0;
            bots_enabled = true;
            lasers.clear();
            players.retain(|_, p| !p.is_bot);
        }

        // 4. Laser duration decay
        lasers.retain_mut(|laser| {
            laser.duration -= 1.0 / (TICK_RATE_HZ as f32);
            laser.duration > 0.0
        });

        // 5. Update respawns and cooldowns
        for session in sessions.values_mut() {
            if session.shoot_cooldown > 0 {
                session.shoot_cooldown -= 1;
            }
        }
        let respawn_ids: Vec<u32> = players
            .iter()
            .filter_map(|(id, player)| if !player.is_alive { Some(*id) } else { None })
            .collect();
        let mut occupied = occupied_cells(&players, None);
        for player_id in respawn_ids {
            if let Some(player) = players.get_mut(&player_id) {
                player.respawn_timer -= 1.0 / (TICK_RATE_HZ as f32);
                if player.respawn_timer <= 0.0 {
                    let (rx, ry) = find_random_spawn(&level, &occupied);
                    player.x = rx as f32 + 0.5;
                    player.y = ry as f32 + 0.5;
                    player.health = 100;
                    player.is_alive = true;
                    occupied.insert((rx, ry));
                }
            }
        }

        // 6. Bot AI ticks (only during play and if bots toggled on)
        if match_state == MatchState::Playing && bots_enabled {
            update_bots(&bot_ids, &mut players, &level, &mut lasers, &mut bot_cooldowns);
        }

        // 7. Broadcast ticks
        tick_id += 1;
        let round_time_left = if match_state == MatchState::Playing {
            round_start_time
                .map(|start_time| (MATCH_DURATION_SECS - now.duration_since(start_time).as_secs_f32()).max(0.0))
                .unwrap_or(MATCH_DURATION_SECS)
        } else {
            0.0
        };
        let tick_data = ServerStateTick {
            tick_id,
            players: players.values().cloned().collect(),
            lasers: lasers.clone(),
            level_index: current_level_idx,
            match_state,
            round_time_left,
            host_id,
            bots_enabled,
        };
        let msg = ServerMessage::Tick(tick_data);
        if let Ok(serialized) = bincode::serialize(&msg) {
            for session in sessions.values() {
                let _ = socket.send_to(&serialized, session.addr);
            }
        }

        let elapsed = now.elapsed();
        if elapsed < tick_duration {
            std::thread::sleep(tick_duration - elapsed);
        }
    }
}

fn handle_client_message(
    msg: &ClientMessage,
    src: SocketAddr,
    socket: &UdpSocket,
    sessions: &mut HashMap<SocketAddr, ClientSession>,
    players: &mut HashMap<u32, PlayerState>,
    next_id: &mut u32,
    level: &mut Level,
    current_level_idx: &mut usize,
    lasers: &mut Vec<LaserEffect>,
    match_state: &mut MatchState,
    round_start_time: &mut Option<Instant>,
    host_id: &mut u32,
    bots_enabled: &mut bool,
) {
    match msg {
        ClientMessage::Join { name } => {
            let session_exists = sessions.contains_key(&src);
            let human_count = players.values().filter(|p| !p.is_bot).count();

            // Reject if 4 players already connected
            if !session_exists && human_count >= 4 {
                let reject = ServerMessage::Reject {
                    reason: "Server Full (Max 4 Players)".to_string(),
                };
                if let Ok(ser) = bincode::serialize(&reject) {
                    let _ = socket.send_to(&ser, src);
                }
                println!("Rejected join from {} (Server full)", src);
                return;
            }

            let session = sessions.entry(src).or_insert_with(|| {
                let player_id = *next_id;
                *next_id += 1;
                println!("Client {} joined from: {}", name, src);
                ClientSession {
                    addr: src,
                    name: name.clone(),
                    player_id,
                    last_packet_time: Instant::now(),
                    shoot_cooldown: 0,
                }
            });

            let is_new_player = !players.contains_key(&session.player_id);

            if is_new_player {
                let occupied = occupied_cells(players, None);
                let (sx, sy) = find_random_spawn(level, &occupied);
                let p = PlayerState {
                    id: session.player_id,
                    name: name.clone(),
                    x: sx as f32 + 0.5,
                    y: sy as f32 + 0.5,
                    dir_idx: 0,
                    score: 0,
                    deaths: 0,
                    health: 100,
                    is_alive: true,
                    is_bot: false,
                    respawn_timer: 0.0,
                };
                players.insert(session.player_id, p);

                // Assign host if first human
                if *host_id == 0 {
                    *host_id = session.player_id;
                    println!("Assign host role to ID: {}", host_id);
                }

                // If in-game and bots enabled, remove a bot to keep max total at 4
                if *match_state == MatchState::Playing && *bots_enabled {
                    if let Some(bot_to_remove) = players.values().filter(|p| p.is_bot).map(|p| p.id).next() {
                        players.remove(&bot_to_remove);
                        println!("Removed bot {} to maintain max 4 players limit.", bot_to_remove);
                    }
                }
            }

            let welcome = ServerMessage::Welcome {
                player_id: session.player_id,
                map_width: level.width,
                map_height: level.height,
                map_cells: level.cells.clone(),
                level_index: *current_level_idx,
                is_host: session.player_id == *host_id,
            };
            if let Ok(serialized) = bincode::serialize(&welcome) {
                let _ = socket.send_to(&serialized, src);
            }
        }
        ClientMessage::Input { action } => {
            if *match_state != MatchState::Playing {
                return;
            }

            if let Some(session) = sessions.get_mut(&src) {
                session.last_packet_time = Instant::now();
                let player_id = session.player_id;
                if let Some(player_snapshot) = players.get(&player_id) {
                    if player_snapshot.is_alive {
                        match action {
                            PlayerAction::MoveForward => {
                                let (dx, dy) = DIR_COORDS[player_snapshot.dir_idx];
                                let nx = player_snapshot.x.floor() as i32 + dx;
                                let ny = player_snapshot.y.floor() as i32 + dy;
                                let can_move = nx >= 0
                                    && nx < level.width as i32
                                    && ny >= 0
                                    && ny < level.height as i32
                                    && !level.cells[ny as usize * level.width + nx as usize]
                                    && !cell_occupied_by_other(players, player_id, nx, ny);
                                if can_move {
                                    if let Some(player) = players.get_mut(&player_id) {
                                        player.x = nx as f32 + 0.5;
                                        player.y = ny as f32 + 0.5;
                                    }
                                }
                            }
                            PlayerAction::MoveBackward => {
                                let (dx, dy) = DIR_COORDS[player_snapshot.dir_idx];
                                let nx = player_snapshot.x.floor() as i32 - dx;
                                let ny = player_snapshot.y.floor() as i32 - dy;
                                let can_move = nx >= 0
                                    && nx < level.width as i32
                                    && ny >= 0
                                    && ny < level.height as i32
                                    && !level.cells[ny as usize * level.width + nx as usize]
                                    && !cell_occupied_by_other(players, player_id, nx, ny);
                                if can_move {
                                    if let Some(player) = players.get_mut(&player_id) {
                                        player.x = nx as f32 + 0.5;
                                        player.y = ny as f32 + 0.5;
                                    }
                                }
                            }
                            PlayerAction::TurnLeft => {
                                if let Some(player) = players.get_mut(&player_id) {
                                    player.dir_idx = (player.dir_idx + 3) % 4;
                                }
                            }
                            PlayerAction::TurnRight => {
                                if let Some(player) = players.get_mut(&player_id) {
                                    player.dir_idx = (player.dir_idx + 1) % 4;
                                }
                            }
                        }
                    }
                }
            }
        }
        ClientMessage::Shoot => {
            if *match_state != MatchState::Playing {
                return;
            }

            if let Some(session) = sessions.get_mut(&src) {
                session.last_packet_time = Instant::now();
                if session.shoot_cooldown == 0 {
                    fire_laser(session.player_id, players, level, lasers);
                    session.shoot_cooldown = 10;
                }
            }
        }
        ClientMessage::Heartbeat => {
            if let Some(session) = sessions.get_mut(&src) {
                session.last_packet_time = Instant::now();
            }
        }
        ClientMessage::Leave => {
            if let Some(session) = sessions.remove(&src) {
                println!("Client {} left", session.name);
                players.remove(&session.player_id);

                if session.player_id == *host_id {
                    *host_id = players.values().filter(|p| !p.is_bot).map(|p| p.id).next().unwrap_or(0);
                    println!("Host left. New Host: {}", host_id);
                }

                // If in-game and bots enabled, spawn a replacement bot
                if *match_state == MatchState::Playing && *bots_enabled {
                    let human_count = players.values().filter(|p| !p.is_bot).count();
                    if human_count > 0 {
                        spawn_replacement_bot(players, level);
                    }
                }
            }
        }
        ClientMessage::RequestLevel { level_idx } => {
            let is_host = sessions.get(&src).map(|s| s.player_id == *host_id).unwrap_or(false);
            if !is_host {
                return;
            }

            if let Some(session) = sessions.get_mut(&src) {
                session.last_packet_time = Instant::now();
            }
            let new_level = if *level_idx == 3 {
                mp::generate_random_maze(19, 19)
            } else {
                mp::get_static_level(*level_idx)
            };

            *level = new_level;
            *current_level_idx = *level_idx;
            if *match_state == MatchState::GameOver {
                begin_new_round(level, players, match_state, round_start_time, lasers, *bots_enabled);
                broadcast_map_update(socket, sessions, level, *current_level_idx);
            } else {
                reset_all_positions(players, level);
                broadcast_map_update(socket, sessions, level, *current_level_idx);
            }
        }
        ClientMessage::BackToLobby => {
            let is_host = sessions.get(&src).map(|s| s.player_id == *host_id).unwrap_or(false);
            if !is_host || *match_state != MatchState::GameOver {
                return;
            }

            if let Some(session) = sessions.get_mut(&src) {
                session.last_packet_time = Instant::now();
            }

            *match_state = MatchState::Lobby;
            *round_start_time = None;
            lasers.clear();
            players.retain(|_, p| !p.is_bot);
            for player in players.values_mut() {
                player.health = 100;
                player.is_alive = true;
                player.respawn_timer = 0.0;
            }
        }
        ClientMessage::StartGame => {
            let is_host = sessions.get(&src).map(|s| s.player_id == *host_id).unwrap_or(false);
            if is_host && *match_state == MatchState::Lobby {
                begin_new_round(level, players, match_state, round_start_time, lasers, *bots_enabled);
                println!("Host started the match!");
            }
        }
        ClientMessage::ToggleBots => {
            let is_host = sessions.get(&src).map(|s| s.player_id == *host_id).unwrap_or(false);
            if is_host && *match_state == MatchState::Lobby {
                *bots_enabled = !*bots_enabled;
                println!("Host toggled bots: {}", *bots_enabled);
            }
        }
    }
}

fn spawn_replacement_bot(players: &mut HashMap<u32, PlayerState>, level: &Level) {
    let active_bot_ids: std::collections::HashSet<u32> = players.values().filter(|p| p.is_bot).map(|p| p.id).collect();
    let mut new_bot_id = 1001;
    for id in 1001..=1004 {
        if !active_bot_ids.contains(&id) {
            new_bot_id = id;
            break;
        }
    }
    let occupied = occupied_cells(players, None);
    let (bx, by) = find_random_spawn(level, &occupied);
    let bot = PlayerState {
        id: new_bot_id,
        name: format!("RetroBot_{}", new_bot_id - 1000),
        x: bx as f32 + 0.5,
        y: by as f32 + 0.5,
        dir_idx: (new_bot_id % 4) as usize,
        score: 0,
        deaths: 0,
        health: 100,
        is_alive: true,
        is_bot: true,
        respawn_timer: 0.0,
    };
    players.insert(new_bot_id, bot);
    println!("Spawned replacement bot {}. Total remains 4.", new_bot_id);
}

fn begin_new_round(
    level: &Level,
    players: &mut HashMap<u32, PlayerState>,
    match_state: &mut MatchState,
    round_start_time: &mut Option<Instant>,
    lasers: &mut Vec<LaserEffect>,
    bots_enabled: bool,
) {
    players.retain(|_, p| !p.is_bot);
    reset_all_positions(players, level);
    for player in players.values_mut() {
        player.score = 0;
        player.deaths = 0;
        player.respawn_timer = 0.0;
    }

    lasers.clear();
    *match_state = MatchState::Playing;
    *round_start_time = Some(Instant::now());

    if bots_enabled {
        let human_count = players.values().filter(|p| !p.is_bot).count();
        let bots_to_spawn = 4usize.saturating_sub(human_count);
        let mut occupied = occupied_cells(players, None);
        for i in 1..=bots_to_spawn {
            let bot_id = 1000 + i as u32;
            let (bx, by) = find_random_spawn(level, &occupied);
            occupied.insert((bx, by));
            let bot = PlayerState {
                id: bot_id,
                name: format!("RetroBot_{}", i),
                x: bx as f32 + 0.5,
                y: by as f32 + 0.5,
                dir_idx: (i % 4) as usize,
                score: 0,
                deaths: 0,
                health: 100,
                is_alive: true,
                is_bot: true,
                respawn_timer: 0.0,
            };
            players.insert(bot_id, bot);
        }
        println!("Spawned {} AI Bots. Total players: 4.", bots_to_spawn);
    }
}

fn broadcast_map_update(
    socket: &UdpSocket,
    sessions: &HashMap<SocketAddr, ClientSession>,
    level: &Level,
    level_idx: usize,
) {
    let msg = ServerMessage::MapUpdate {
        map_width: level.width,
        map_height: level.height,
        map_cells: level.cells.clone(),
        level_index: level_idx,
    };
    if let Ok(serialized) = bincode::serialize(&msg) {
        for session in sessions.values() {
            let _ = socket.send_to(&serialized, session.addr);
        }
    }
}
