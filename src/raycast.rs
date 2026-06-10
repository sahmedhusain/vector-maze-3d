#[derive(Debug, Clone, Copy)]
pub struct RaycastResult {
    pub depth: f32,
    pub hit_x: f32,
    pub hit_y: f32,
    pub map_x: i32,
    pub map_y: i32,
    pub side: usize,
}

pub fn raycast(
    px: f32,
    py: f32,
    angle: f32,
    map_cells: &[bool],
    map_width: usize,
    map_height: usize,
) -> Option<RaycastResult> {
    let ray_dir_x = angle.cos();
    let ray_dir_y = angle.sin();

    let mut map_x = px.floor() as i32;
    let mut map_y = py.floor() as i32;

    let delta_dist_x = if ray_dir_x.abs() < 1e-6 {
        1e30
    } else {
        (1.0 / ray_dir_x).abs()
    };
    let delta_dist_y = if ray_dir_y.abs() < 1e-6 {
        1e30
    } else {
        (1.0 / ray_dir_y).abs()
    };

    let step_x: i32;
    let step_y: i32;
    let mut side_dist_x: f32;
    let mut side_dist_y: f32;

    if ray_dir_x < 0.0 {
        step_x = -1;
        side_dist_x = (px - map_x as f32) * delta_dist_x;
    } else {
        step_x = 1;
        side_dist_x = (map_x as f32 + 1.0 - px) * delta_dist_x;
    }

    if ray_dir_y < 0.0 {
        step_y = -1;
        side_dist_y = (py - map_y as f32) * delta_dist_y;
    } else {
        step_y = 1;
        side_dist_y = (map_y as f32 + 1.0 - py) * delta_dist_y;
    }

    let mut hit = false;
    let mut side = 0;

    for _ in 0..100 {
        if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            side = 0;
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            side = 1;
        }

        if map_x < 0 || map_x >= map_width as i32 || map_y < 0 || map_y >= map_height as i32 {
            break;
        }

        if map_cells[map_y as usize * map_width + map_x as usize] {
            hit = true;
            break;
        }
    }

    if hit {
        let depth = if side == 0 {
            side_dist_x - delta_dist_x
        } else {
            side_dist_y - delta_dist_y
        };
        let hit_x = px + ray_dir_x * depth;
        let hit_y = py + ray_dir_y * depth;
        Some(RaycastResult {
            depth,
            hit_x,
            hit_y,
            map_x,
            map_y,
            side,
        })
    } else {
        None
    }
}
