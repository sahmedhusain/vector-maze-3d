use std::net::{SocketAddr, UdpSocket};
use macroquad::prelude::*;
use mp::ClientMessage;

use crate::{draw_text_centered, draw_text_ex_font};

pub fn run_editor_tick(
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
    socket: &UdpSocket,
    server_addr: SocketAddr,
    cells: &mut Vec<bool>,
    map_w: &mut usize,
    map_h: &mut usize,
    font: &Font,
) {
    draw_rectangle(view_x, view_y, view_w, view_h, Color::new(0.03, 0.05, 0.08, 1.0));

    let w = *map_w;
    let h = *map_h;

    let grid_area_size = (view_h - 120.0).min(view_w - 60.0).max(200.0);
    let cell_sz = grid_area_size / w.max(h) as f32;

    let start_x = view_x + (view_w - cell_sz * w as f32) / 2.0;
    let start_y = view_y + (view_h - cell_sz * h as f32 - 40.0) / 2.0;

    draw_text_ex_font("LEFT CLICK: Draw Wall  |  RIGHT CLICK: Clear Wall", view_x + 20.0, view_y + 25.0, 14.0, YELLOW, font);

    for r in 0..h {
        for c in 0..w {
            let rx = start_x + c as f32 * cell_sz;
            let ry = start_y + r as f32 * cell_sz;

            let is_outer = r == 0 || r == h - 1 || c == 0 || c == w - 1;

            let cell_col = if cells[r * w + c] {
                if is_outer { Color::new(0.25, 0.30, 0.40, 1.0) } else { Color::new(0.0, 1.0, 1.0, 1.0) }
            } else {
                Color::new(0.05, 0.07, 0.12, 1.0)
            };

            draw_rectangle(rx, ry, cell_sz, cell_sz, cell_col);
            draw_rectangle_lines(rx, ry, cell_sz, cell_sz, 0.5, Color::new(0.15, 0.20, 0.30, 0.4));
        }
    }

    let (mx, my) = mouse_position();
    let mouse_in_grid = mx >= start_x && mx < start_x + w as f32 * cell_sz && my >= start_y && my < start_y + h as f32 * cell_sz;

    if mouse_in_grid {
        let grid_c = ((mx - start_x) / cell_sz) as usize;
        let grid_r = ((my - start_y) / cell_sz) as usize;

        let is_outer = grid_r == 0 || grid_r == h - 1 || grid_c == 0 || grid_c == w - 1;

        if !is_outer {
            if is_mouse_button_down(MouseButton::Left) {
                cells[grid_r * w + grid_c] = true;
            } else if is_mouse_button_down(MouseButton::Right) {
                cells[grid_r * w + grid_c] = false;
            }
        }
    }

    let btn_y = view_y + view_h - 45.0;
    
    let r_btn = draw_button("R: RANDOM DFS MAZE", view_x + 30.0, btn_y, 160.0, 30.0, font);
    let c_btn = draw_button("C: CLEAR GRID", view_x + 210.0, btn_y, 130.0, 30.0, font);
    let u_btn = draw_button("U: UPLOAD TO SERVER", view_x + 360.0, btn_y, 180.0, 30.0, font);

    let mut click_action = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        if r_btn {
            click_action = Some('R');
        } else if c_btn {
            click_action = Some('C');
        } else if u_btn {
            click_action = Some('U');
        }
    }

    if is_key_pressed(KeyCode::R) || click_action == Some('R') {
        let generated = mp::generate_random_maze(w, h);
        *cells = generated.cells;
        *map_w = generated.width;
        *map_h = generated.height;
    } else if is_key_pressed(KeyCode::C) || click_action == Some('C') {
        for r in 0..h {
            for c in 0..w {
                let is_outer = r == 0 || r == h - 1 || c == 0 || c == w - 1;
                cells[r * w + c] = is_outer;
            }
        }
    } else if is_key_pressed(KeyCode::U) || click_action == Some('U') {
        let custom_pkt = ClientMessage::CustomMap {
            width: w,
            height: h,
            cells: cells.clone(),
        };
        if let Ok(ser) = bincode::serialize(&custom_pkt) {
            let _ = socket.send_to(&ser, server_addr);
        }
        println!("Custom map sent to server!");
    }
}

fn draw_button(text: &str, x: f32, y: f32, w: f32, h: f32, font: &Font) -> bool {
    let (mx, my) = mouse_position();
    let hover = mx >= x && mx <= x + w && my >= y && my <= y + h;

    let bg_col = if hover { Color::new(0.0, 0.8, 0.7, 0.9) } else { Color::new(0.06, 0.12, 0.22, 0.9) };
    let border_col = if hover { WHITE } else { Color::new(0.1, 0.2, 0.35, 1.0) };
    let text_col = if hover { BLACK } else { WHITE };

    draw_rectangle(x, y, w, h, bg_col);
    draw_rectangle_lines(x, y, w, h, 1.5, border_col);
    draw_text_centered(text, x + w / 2.0, y + h / 2.0 + 4.0, 12.0, text_col, font);

    hover
}
