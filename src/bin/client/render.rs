use std::collections::HashMap;
use macroquad::prelude::*;
use mp::LaserEffect;

use crate::{VisualPlayer, draw_text_centered};

pub fn render_3d_viewport(
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
    local_player: &VisualPlayer,
    players: &HashMap<u32, VisualPlayer>,
    map_cells: &[bool],
    map_width: usize,
    map_height: usize,
    lasers: &[LaserEffect],
    font: &Font,
) {
    draw_rectangle(view_x, view_y, view_w, view_h / 2.0, Color::new(0.02, 0.03, 0.05, 1.0));
    draw_rectangle(view_x, view_y + view_h / 2.0, view_w, view_h / 2.0, Color::new(0.05, 0.06, 0.08, 1.0));

    let num_rays = 320;
    let col_w = view_w / num_rays as f32;
    let fov = 60.0f32.to_radians();
    let focal_len = (num_rays as f32 / 2.0) / (fov / 2.0).tan();

    let mut depths = vec![999.0; num_rays];
    let mut y_tops = vec![0.0; num_rays];
    let mut y_bottoms = vec![0.0; num_rays];
    let mut sides = vec![0; num_rays];
    let mut hit_coords = vec![(0, 0); num_rays];

    for x in 0..num_rays {
        let ray_angle = local_player.vtheta + ((x as f32 - num_rays as f32 / 2.0)).atan2(focal_len);
        if let Some(res) = mp::raycast(local_player.vx, local_player.vy, ray_angle, map_cells, map_width, map_height) {
            let corrected_depth = res.depth * (ray_angle - local_player.vtheta).cos();
            depths[x] = res.depth;
            sides[x] = res.side;
            hit_coords[x] = (res.map_x, res.map_y);

            let line_height = (view_h / corrected_depth) * 0.95;
            let y_top = view_y + (view_h - line_height) / 2.0;
            let y_bottom = view_y + (view_h + line_height) / 2.0;

            y_tops[x] = y_top;
            y_bottoms[x] = y_bottom;

            let shade = 1.0 / (1.0 + corrected_depth * 0.15);
            let intensity = if res.side == 1 { 0.7 } else { 1.0 };
            
            let r = 0.04 * shade * intensity;
            let g = 0.10 * shade * intensity;
            let b = 0.18 * shade * intensity;
            let wall_color = Color::new(r, g, b, 0.9);

            let draw_top = y_top.max(view_y);
            let draw_bottom = y_bottom.min(view_y + view_h);
            draw_rectangle(view_x + x as f32 * col_w, draw_top, col_w, draw_bottom - draw_top, wall_color);
        }
    }

    for x in 1..num_rays {
        if depths[x] > 18.0 && depths[x-1] > 18.0 {
            continue;
        }

        let diff = (depths[x] - depths[x-1]).abs();
        if diff < 0.6 && hit_coords[x] == hit_coords[x-1] && sides[x] == sides[x-1] {
            let alpha = (1.0 / (1.0 + depths[x] * 0.12)).min(1.0);
            let border_color = Color::new(0.0, 1.0, 0.85, alpha);

            draw_line(
                view_x + (x - 1) as f32 * col_w, y_tops[x-1],
                view_x + x as f32 * col_w, y_tops[x],
                1.5, border_color,
            );
            draw_line(
                view_x + (x - 1) as f32 * col_w, y_bottoms[x-1],
                view_x + x as f32 * col_w, y_bottoms[x],
                1.5, border_color,
            );
        } else {
            let alpha = (1.0 / (1.0 + depths[x].min(depths[x-1]) * 0.12)).min(1.0);
            let border_color = Color::new(0.0, 1.0, 0.85, alpha);

            if depths[x] < depths[x-1] {
                draw_line(view_x + x as f32 * col_w, y_tops[x].max(view_y), view_x + x as f32 * col_w, y_bottoms[x].min(view_y + view_h), 1.5, border_color);
            } else {
                draw_line(view_x + x as f32 * col_w, y_tops[x-1].max(view_y), view_x + x as f32 * col_w, y_bottoms[x-1].min(view_y + view_h), 1.5, border_color);
            }
        }
    }

    let mut visible_entities = Vec::new();
    for p in players.values() {
        if p.id == local_player.id || !p.is_alive {
            continue;
        }

        let dx = p.vx - local_player.vx;
        let dy = p.vy - local_player.vy;

        let rx = -dx * local_player.vtheta.sin() + dy * local_player.vtheta.cos();
        let rz = dx * local_player.vtheta.cos() + dy * local_player.vtheta.sin();

        if rz > 0.1 {
            visible_entities.push((p, rx, rz));
        }
    }

    visible_entities.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    for (p, rx, rz) in visible_entities {
        let sx = (view_x + view_w / 2.0) + (rx / rz) * focal_len;
        let sy = view_y + view_h / 2.0;

        let col_idx = (((sx - view_x) / view_w) * num_rays as f32) as i32;
        if col_idx >= 0 && col_idx < num_rays as i32 {
            if rz < depths[col_idx as usize] + 0.3 {
                let p_size = (view_h / rz) * 0.42;

                if sx + p_size/2.0 >= view_x && sx - p_size/2.0 <= view_x + view_w {
                    let alpha = (1.0 / (1.0 + rz * 0.08)).min(1.0);
                    let shape_color = if p.is_bot {
                        Color::new(1.0, 0.5, 0.0, alpha)
                    } else {
                        Color::new(1.0, 0.2, 0.4, alpha)
                    };

                    let t_pt = vec2(sx, sy - p_size/2.0);
                    let b_pt = vec2(sx, sy + p_size/2.0);
                    let l_pt = vec2(sx - p_size/2.0, sy);
                    let r_pt = vec2(sx + p_size/2.0, sy);

                    draw_line(t_pt.x, t_pt.y, r_pt.x, r_pt.y, 2.0, shape_color);
                    draw_line(r_pt.x, r_pt.y, b_pt.x, b_pt.y, 2.0, shape_color);
                    draw_line(b_pt.x, b_pt.y, l_pt.x, l_pt.y, 2.0, shape_color);
                    draw_line(l_pt.x, l_pt.y, t_pt.x, t_pt.y, 2.0, shape_color);
                    draw_line(t_pt.x, t_pt.y, b_pt.x, b_pt.y, 1.0, shape_color);
                    draw_line(l_pt.x, l_pt.y, r_pt.x, r_pt.y, 1.0, shape_color);

                    draw_circle(sx, sy, (p_size * 0.12).max(2.0), Color::new(0.0, 1.0, 0.85, alpha));

                    let ny = sy - p_size/2.0 - 15.0;
                    if ny >= view_y {
                        draw_text_centered(&p.name, sx, ny, 12.0, WHITE, font);
                        
                        let hb_w = p_size * 0.8;
                        let hb_h = 3.0;
                        let hb_x = sx - hb_w / 2.0;
                        let hb_y = ny + 4.0;
                        draw_rectangle(hb_x, hb_y, hb_w, hb_h, Color::new(0.3, 0.0, 0.0, alpha));
                        let green_w = hb_w * (p.health as f32 / 100.0);
                        draw_rectangle(hb_x, hb_y, green_w, hb_h, Color::new(0.0, 0.8, 0.2, alpha));
                    }
                }
            }
        }
    }

    for laser in lasers {
        let dx1 = laser.from_x - local_player.vx;
        let dy1 = laser.from_y - local_player.vy;
        let rx1 = -dx1 * local_player.vtheta.sin() + dy1 * local_player.vtheta.cos();
        let rz1 = dx1 * local_player.vtheta.cos() + dy1 * local_player.vtheta.sin();

        let dx2 = laser.to_x - local_player.vx;
        let dy2 = laser.to_y - local_player.vy;
        let rx2 = -dx2 * local_player.vtheta.sin() + dy2 * local_player.vtheta.cos();
        let rz2 = dx2 * local_player.vtheta.cos() + dy2 * local_player.vtheta.sin();

        let mut z1 = rz1;
        let mut x1 = rx1;
        let mut z2 = rz2;
        let mut x2 = rx2;

        if z1 < 0.1 && z2 < 0.1 {
            continue;
        }

        if z1 < 0.1 {
            let t = (0.1 - z1) / (z2 - z1);
            x1 = x1 + t * (x2 - x1);
            z1 = 0.1;
        } else if z2 < 0.1 {
            let t = (0.1 - z2) / (z1 - z2);
            x2 = x2 + t * (x1 - x2);
            z2 = 0.1;
        }

        let sx1 = (view_x + view_w / 2.0) + (x1 / z1) * focal_len;
        let sy1 = view_y + view_h / 2.0;
        let sx2 = (view_x + view_w / 2.0) + (x2 / z2) * focal_len;
        let sy2 = view_y + view_h / 2.0;

        let alpha = laser.duration / 0.2;
        let laser_color = Color::new(1.0, 0.1, 0.1, alpha);

        draw_line(sx1, sy1, sx2, sy2, 3.0, laser_color);
    }

    let cx = view_x + view_w / 2.0;
    let cy = view_y + view_h / 2.0;
    draw_line(cx - 8.0, cy, cx + 8.0, cy, 1.0, Color::new(0.0, 1.0, 0.8, 0.5));
    draw_line(cx, cy - 8.0, cx, cy + 8.0, 1.0, Color::new(0.0, 1.0, 0.8, 0.5));
}
