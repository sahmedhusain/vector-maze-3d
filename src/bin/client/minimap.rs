use macroquad::prelude::*;
use mp::LaserEffect;
use crate::VisualPlayer;

pub fn render_minimap(
    mx: f32,
    my: f32,
    mw: f32,
    mh: f32,
    map_w: usize,
    map_h: usize,
    cells: &[bool],
    my_id: u32,
    players: &std::collections::HashMap<u32, VisualPlayer>,
    lasers: &[LaserEffect],
) {
    let cell_w = mw / map_w as f32;
    let cell_h = mh / map_h as f32;
    let cell_sz = cell_w.min(cell_h);

    let offset_x = (mw - cell_sz * map_w as f32) / 2.0;
    let offset_y = (mh - cell_sz * map_h as f32) / 2.0;

    let start_x = mx + offset_x;
    let start_y = my + offset_y;

    for r in 0..map_h {
        for c in 0..map_w {
            let rx = start_x + c as f32 * cell_sz;
            let ry = start_y + r as f32 * cell_sz;

            if cells[r * map_w + c] {
                draw_rectangle(rx, ry, cell_sz, cell_sz, Color::new(0.12, 0.16, 0.24, 1.0));
                draw_rectangle_lines(rx, ry, cell_sz, cell_sz, 0.5, Color::new(0.05, 0.08, 0.12, 0.8));
            } else {
                draw_rectangle(rx, ry, cell_sz, cell_sz, Color::new(0.02, 0.03, 0.05, 1.0));
                draw_rectangle_lines(rx, ry, cell_sz, cell_sz, 0.5, Color::new(0.08, 0.1, 0.15, 0.5));
            }
        }
    }

    for laser in lasers {
        let lx1 = start_x + laser.from_x * cell_sz;
        let ly1 = start_y + laser.from_y * cell_sz;
        let lx2 = start_x + laser.to_x * cell_sz;
        let ly2 = start_y + laser.to_y * cell_sz;

        let alpha = laser.duration / 0.2;
        draw_line(lx1, ly1, lx2, ly2, 1.5, Color::new(1.0, 0.1, 0.1, alpha));
    }

    for p in players.values() {
        if !p.is_alive {
            continue;
        }

        let px = start_x + p.vx * cell_sz;
        let py = start_y + p.vy * cell_sz;
        let rad = (cell_sz * 0.35).max(3.0);

        if p.id == my_id {
            let tri_h = rad * 1.5;
            let p1 = vec2(px + tri_h * p.vtheta.cos(), py + tri_h * p.vtheta.sin());
            let p2 = vec2(px + rad * (p.vtheta + 2.3).cos(), py + rad * (p.vtheta + 2.3).sin());
            let p3 = vec2(px + rad * (p.vtheta - 2.3).cos(), py + rad * (p.vtheta - 2.3).sin());

            draw_triangle(p1, p2, p3, GREEN);
        } else {
            let col = if p.is_bot { ORANGE } else { RED };
            draw_circle(px, py, rad, col);

            let nx = px + rad * 1.2 * p.vtheta.cos();
            let ny = py + rad * 1.2 * p.vtheta.sin();
            draw_line(px, py, nx, ny, 1.0, WHITE);
        }
    }
}
