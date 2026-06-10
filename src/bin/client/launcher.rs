use macroquad::prelude::*;
use mp::DEFAULT_PORT;
use std::io::Write;

pub struct HistoryItem {
    pub alias: String,
    pub ip: String,
}

pub enum FocusedField {
    Ip,
    Username,
    Alias,
}

pub struct LauncherState {
    pub ip_input: String,
    pub username_input: String,
    pub alias_input: String,
    pub focused_field: FocusedField,
    pub history: Vec<HistoryItem>,
    pub error_msg: Option<String>,
    pub backspace_timer: f32,
}

impl LauncherState {
    pub fn new() -> Self {
        let mut history = Vec::new();
        let history_file = "hosts_history.txt";
        if let Ok(content) = std::fs::read_to_string(history_file) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    history.push(HistoryItem {
                        alias: parts[0].trim().to_string(),
                        ip: parts[1].trim().to_string(),
                    });
                } else if !line.trim().is_empty() {
                    history.push(HistoryItem {
                        alias: line.trim().to_string(),
                        ip: line.trim().to_string(),
                    });
                }
            }
        }
        Self {
            ip_input: String::new(),
            username_input: String::new(),
            alias_input: String::new(),
            focused_field: FocusedField::Ip,
            history,
            error_msg: None,
            backspace_timer: 0.0,
        }
    }

    pub fn save_to_history(&mut self, ip: &str, alias: &str) {
        self.history.retain(|item| item.ip != ip);
        let save_alias = if alias.is_empty() { ip.to_string() } else { alias.to_string() };
        self.history.insert(0, HistoryItem {
            alias: save_alias,
            ip: ip.to_string(),
        });
        self.write_history_file();
    }

    pub fn delete_from_history(&mut self, idx: usize) {
        if idx < self.history.len() {
            self.history.remove(idx);
            self.write_history_file();
        }
    }

    fn write_history_file(&self) {
        let history_file = "hosts_history.txt";
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(history_file)
        {
            for item in &self.history {
                let _ = writeln!(file, "{}={}", item.alias, item.ip);
            }
        }
    }
}

fn draw_text_custom(text: &str, x: f32, y: f32, size: f32, color: Color, font: &Font) {
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

fn draw_text_centered_custom(text: &str, cx: f32, cy: f32, size: f32, color: Color, font: &Font) {
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

pub fn update_and_draw_launcher(state: &mut LauncherState, dt: f32, font: &Font) -> Option<(String, String)> {
    if is_key_down(KeyCode::Backspace) {
        state.backspace_timer += dt;
        if is_key_pressed(KeyCode::Backspace) || state.backspace_timer > 0.4 {
            if state.backspace_timer > 0.4 {
                state.backspace_timer = 0.35;
            }
            match state.focused_field {
                FocusedField::Ip => { state.ip_input.pop(); }
                FocusedField::Username => { state.username_input.pop(); }
                FocusedField::Alias => { state.alias_input.pop(); }
            }
        }
    } else {
        state.backspace_timer = 0.0;
    }

    if is_key_pressed(KeyCode::Tab) {
        state.focused_field = match state.focused_field {
            FocusedField::Ip => FocusedField::Username,
            FocusedField::Username => FocusedField::Alias,
            FocusedField::Alias => FocusedField::Ip,
        };
    }

    let mut try_connect = is_key_pressed(KeyCode::Enter);

    while let Some(c) = get_char_pressed() {
        if c.is_ascii_graphic() || c == ' ' {
            match state.focused_field {
                FocusedField::Ip => {
                    if state.ip_input.len() < 30 {
                        state.ip_input.push(c);
                    }
                }
                FocusedField::Username => {
                    if state.username_input.len() < 16 {
                        state.username_input.push(c);
                    }
                }
                FocusedField::Alias => {
                    if state.alias_input.len() < 20 {
                        state.alias_input.push(c);
                    }
                }
            }
        }
    }

    let screen_w = screen_width();
    let screen_h = screen_height();

    clear_background(Color::new(0.02, 0.04, 0.08, 1.0));

    for x in (0..(screen_w as i32)).step_by(80) {
        draw_line(x as f32, 0.0, x as f32, screen_h, 1.0, Color::new(0.05, 0.1, 0.2, 0.2));
    }
    for y in (0..(screen_h as i32)).step_by(80) {
        draw_line(0.0, y as f32, screen_w, y as f32, 1.0, Color::new(0.05, 0.1, 0.2, 0.2));
    }

    let title = "MAZE WARS 3D";
    draw_text_centered_custom(title, screen_w / 2.0, screen_h * 0.1, 50.0, Color::new(0.0, 1.0, 1.0, 1.0), font);

    let subtitle = "MULTIPLAYER LAUNCHER";
    draw_text_centered_custom(subtitle, screen_w / 2.0, screen_h * 0.1 + 50.0, 20.0, GRAY, font);

    let (mx, my) = mouse_position();
    let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);

    let use_side_by_side = screen_w > 900.0;

    let fx;
    let fy;
    let fw;
    let fh;

    let hx;
    let hy;
    let hw;
    let hh;

    if use_side_by_side {
        fw = (screen_w * 0.42).clamp(320.0, 480.0);
        fh = (screen_h * 0.65).clamp(400.0, 560.0);
        let gap = (screen_w - 2.0 * fw) / 3.0;

        fx = gap;
        fy = screen_h * 0.22;

        hw = fw;
        hh = fh;
        hx = 2.0 * gap + fw;
        hy = fy;
    } else {
        fw = (screen_w - 60.0).min(480.0);
        fh = 360.0;
        fx = (screen_w - fw) / 2.0;
        fy = 150.0;

        hw = fw;
        hh = 200.0;
        hx = fx;
        hy = fy + fh + 20.0;
    }

    draw_rectangle(fx, fy, fw, fh, Color::new(0.05, 0.08, 0.15, 0.75));
    draw_rectangle_lines(fx, fy, fw, fh, 2.0, Color::new(0.1, 0.2, 0.4, 0.5));
    draw_text_custom("CONNECTION SETTINGS", fx + 20.0, fy + 35.0, 20.0, Color::new(0.0, 1.0, 1.0, 1.0), font);

    let field_spacing = (fh * 0.18).clamp(55.0, 85.0);
    let ip_rect_y = fy + 70.0;
    let user_rect_y = ip_rect_y + field_spacing;
    let alias_rect_y = user_rect_y + field_spacing;
    let btn_y = alias_rect_y + field_spacing;

    draw_text_custom("SERVER IP ADDRESS (ip:port)", fx + 20.0, ip_rect_y - 8.0, 14.0, LIGHTGRAY, font);
    let ip_border_color = match state.focused_field {
        FocusedField::Ip => Color::new(0.0, 1.0, 1.0, 0.8),
        _ => Color::new(0.1, 0.2, 0.3, 0.8),
    };
    draw_rectangle(fx + 20.0, ip_rect_y, fw - 40.0, 40.0, Color::new(0.02, 0.03, 0.06, 0.9));
    draw_rectangle_lines(fx + 20.0, ip_rect_y, fw - 40.0, 40.0, 1.5, ip_border_color);

    if mouse_pressed && mx >= fx + 20.0 && mx <= fx + fw - 20.0 && my >= ip_rect_y && my <= ip_rect_y + 40.0 {
        state.focused_field = FocusedField::Ip;
    }

    draw_text_custom(&state.ip_input, fx + 30.0, ip_rect_y + 25.0, 16.0, WHITE, font);
    if let FocusedField::Ip = state.focused_field {
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = fx + 32.0 + measure_text(&state.ip_input, Some(font), 16, 1.0).width;
            draw_line(cursor_x, ip_rect_y + 10.0, cursor_x, ip_rect_y + 30.0, 1.5, Color::new(0.0, 1.0, 1.0, 1.0));
        }
    }

    draw_text_custom("PLAYER USERNAME", fx + 20.0, user_rect_y - 8.0, 14.0, LIGHTGRAY, font);
    let user_border_color = match state.focused_field {
        FocusedField::Username => Color::new(0.0, 1.0, 1.0, 0.8),
        _ => Color::new(0.1, 0.2, 0.3, 0.8),
    };
    draw_rectangle(fx + 20.0, user_rect_y, fw - 40.0, 40.0, Color::new(0.02, 0.03, 0.06, 0.9));
    draw_rectangle_lines(fx + 20.0, user_rect_y, fw - 40.0, 40.0, 1.5, user_border_color);

    if mouse_pressed && mx >= fx + 20.0 && mx <= fx + fw - 20.0 && my >= user_rect_y && my <= user_rect_y + 40.0 {
        state.focused_field = FocusedField::Username;
    }

    draw_text_custom(&state.username_input, fx + 30.0, user_rect_y + 25.0, 16.0, WHITE, font);
    if let FocusedField::Username = state.focused_field {
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = fx + 32.0 + measure_text(&state.username_input, Some(font), 16, 1.0).width;
            draw_line(cursor_x, user_rect_y + 10.0, cursor_x, user_rect_y + 30.0, 1.5, Color::new(0.0, 1.0, 1.0, 1.0));
        }
    }

    draw_text_custom("SERVER ALIAS (OPTIONAL)", fx + 20.0, alias_rect_y - 8.0, 14.0, LIGHTGRAY, font);
    let alias_border_color = match state.focused_field {
        FocusedField::Alias => Color::new(0.0, 1.0, 1.0, 0.8),
        _ => Color::new(0.1, 0.2, 0.3, 0.8),
    };
    draw_rectangle(fx + 20.0, alias_rect_y, fw - 40.0, 40.0, Color::new(0.02, 0.03, 0.06, 0.9));
    draw_rectangle_lines(fx + 20.0, alias_rect_y, fw - 40.0, 40.0, 1.5, alias_border_color);

    if mouse_pressed && mx >= fx + 20.0 && mx <= fx + fw - 20.0 && my >= alias_rect_y && my <= alias_rect_y + 40.0 {
        state.focused_field = FocusedField::Alias;
    }

    draw_text_custom(&state.alias_input, fx + 30.0, alias_rect_y + 25.0, 16.0, WHITE, font);
    if let FocusedField::Alias = state.focused_field {
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = fx + 32.0 + measure_text(&state.alias_input, Some(font), 16, 1.0).width;
            draw_line(cursor_x, alias_rect_y + 10.0, cursor_x, alias_rect_y + 30.0, 1.5, Color::new(0.0, 1.0, 1.0, 1.0));
        }
    }

    let is_hover_btn = mx >= fx + 20.0 && mx <= fx + fw - 20.0 && my >= btn_y && my <= btn_y + 50.0;
    let btn_bg = if is_hover_btn {
        Color::new(0.0, 0.6, 0.6, 1.0)
    } else {
        Color::new(0.0, 0.4, 0.4, 1.0)
    };
    draw_rectangle(fx + 20.0, btn_y, fw - 40.0, 50.0, btn_bg);
    draw_rectangle_lines(fx + 20.0, btn_y, fw - 40.0, 50.0, 2.0, Color::new(0.0, 1.0, 1.0, 1.0));

    let btn_lbl = "CONNECT";
    draw_text_centered_custom(btn_lbl, fx + fw / 2.0, btn_y + 32.0, 20.0, WHITE, font);

    if mouse_pressed && is_hover_btn {
        try_connect = true;
    }

    if let Some(err) = &state.error_msg {
        draw_text_centered_custom(err, fx + fw / 2.0, btn_y + 75.0, 14.0, RED, font);
    }

    draw_rectangle(hx, hy, hw, hh, Color::new(0.05, 0.08, 0.15, 0.75));
    draw_rectangle_lines(hx, hy, hw, hh, 2.0, Color::new(0.1, 0.2, 0.4, 0.5));
    draw_text_custom("SAVED SERVERS HISTORY", hx + 20.0, hy + 35.0, 20.0, Color::new(0.0, 1.0, 1.0, 1.0), font);

    let mut delete_clicked: Option<usize> = None;
    let mut select_clicked: Option<usize> = None;

    let start_y = hy + 60.0;
    let row_h = 45.0;
    let max_rows = if use_side_by_side { ((hh - 80.0) / 53.0) as usize } else { 2 };
    for (idx, item) in state.history.iter().enumerate().take(max_rows) {
        let ry = start_y + (idx as f32) * (row_h + 8.0);
        let rx = hx + 20.0;
        let rw = hw - 40.0;

        let is_hover_row = mx >= rx && mx <= rx + rw && my >= ry && my <= ry + row_h;
        let row_bg = if is_hover_row {
            Color::new(0.1, 0.18, 0.3, 0.8)
        } else {
            Color::new(0.02, 0.04, 0.08, 0.6)
        };
        draw_rectangle(rx, ry, rw, row_h, row_bg);
        draw_rectangle_lines(rx, ry, rw, row_h, 1.0, Color::new(0.1, 0.2, 0.4, 0.3));

        draw_text_custom(&item.alias, rx + 15.0, ry + 20.0, 14.0, WHITE, font);
        draw_text_custom(&item.ip, rx + 15.0, ry + 36.0, 12.0, GRAY, font);

        let del_x = rx + rw - 35.0;
        let del_y = ry + 10.0;
        let del_w = 25.0;
        let del_h = 25.0;
        let is_hover_del = mx >= del_x && mx <= del_x + del_w && my >= del_y && my <= del_y + del_h;
        let del_color = if is_hover_del { RED } else { Color::new(0.6, 0.2, 0.2, 1.0) };
        draw_rectangle(del_x, del_y, del_w, del_h, Color::new(0.01, 0.02, 0.04, 0.8));
        draw_rectangle_lines(del_x, del_y, del_w, del_h, 1.0, del_color);
        draw_text_custom("X", del_x + 8.0, del_y + 17.0, 12.0, del_color, font);

        if mouse_pressed {
            if is_hover_del {
                delete_clicked = Some(idx);
            } else if is_hover_row {
                select_clicked = Some(idx);
            }
        }
    }

    if state.history.is_empty() {
        draw_text_custom("No connection history found.", hx + 30.0, start_y + 30.0, 14.0, GRAY, font);
    }

    if let Some(idx) = delete_clicked {
        state.delete_from_history(idx);
    } else if let Some(idx) = select_clicked {
        let selected = &state.history[idx];
        state.ip_input = selected.ip.clone();
        state.alias_input = selected.alias.clone();
        state.focused_field = FocusedField::Username;
        state.error_msg = None;
    }

    if try_connect {
        let ip = state.ip_input.trim().to_string();
        let name = state.username_input.trim().to_string();
        let alias = state.alias_input.trim().to_string();
        if name.is_empty() {
            state.error_msg = Some("Username cannot be empty!".to_string());
        } else if ip.is_empty() {
            state.error_msg = Some("IP Address cannot be empty!".to_string());
        } else {
            let mut final_ip = ip;
            if !final_ip.contains(':') {
                final_ip = format!("{}:{}", final_ip, DEFAULT_PORT);
            }
            if final_ip.parse::<std::net::SocketAddr>().is_err() {
                state.error_msg = Some("Invalid IP address formatting!".to_string());
            } else {
                state.save_to_history(&final_ip, &alias);
                return Some((final_ip, name));
            }
        }
    }

    None
}
