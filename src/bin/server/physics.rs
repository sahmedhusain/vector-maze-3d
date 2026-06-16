use std::collections::HashMap;
use std::collections::HashSet;
use rand::seq::SliceRandom;
use rand::thread_rng;
use mp::*;

pub fn cell_occupied_by_other(
    players: &HashMap<u32, PlayerState>,
    exclude_id: u32,
    cell_x: i32,
    cell_y: i32,
) -> bool {
    players.values().any(|player| {
        player.id != exclude_id
            && player.is_alive
            && player.x.floor() as i32 == cell_x
            && player.y.floor() as i32 == cell_y
    })
}

pub fn occupied_cells(players: &HashMap<u32, PlayerState>, exclude_id: Option<u32>) -> HashSet<(usize, usize)> {
    let mut occupied = HashSet::new();
    for player in players.values() {
        if exclude_id.map(|id| id == player.id).unwrap_or(false) {
            continue;
        }
        if player.is_alive {
            occupied.insert((player.x.floor() as usize, player.y.floor() as usize));
        }
    }
    occupied
}

pub fn check_line_of_sight(
    self_id: u32,
    sx: f32,
    sy: f32,
    dir_idx: usize,
    players: &HashMap<u32, PlayerState>,
    level: &Level,
) -> (Option<u32>, f32) {
    let (dx, dy) = DIR_COORDS[dir_idx];
    let start_x = sx.floor() as i32;
    let start_y = sy.floor() as i32;

    let mut steps = 1;
    loop {
        let cx = start_x + dx * steps;
        let cy = start_y + dy * steps;

        if cx < 0 || cx >= level.width as i32 || cy < 0 || cy >= level.height as i32 {
            break;
        }
        if level.cells[cy as usize * level.width + cx as usize] {
            break;
        }

        for other in players.values() {
            if other.id != self_id && other.is_alive {
                let ox = other.x.floor() as i32;
                let oy = other.y.floor() as i32;
                if ox == cx && oy == cy {
                    return (Some(other.id), steps as f32);
                }
            }
        }
        steps += 1;
    }
    (None, 999.0)
}

pub fn fire_laser(
    shooter_id: u32,
    players: &mut HashMap<u32, PlayerState>,
    level: &Level,
    lasers: &mut Vec<LaserEffect>,
) {
    let shooter = match players.get(&shooter_id) {
        Some(s) if s.is_alive => s.clone(),
        _ => return,
    };

    let (dx, dy) = DIR_COORDS[shooter.dir_idx];
    let start_x = shooter.x.floor() as i32;
    let start_y = shooter.y.floor() as i32;

    let mut hit_x = shooter.x;
    let mut hit_y = shooter.y;
    let mut hit_player_id = None;

    let mut steps = 1;
    loop {
        let cx = start_x + dx * steps;
        let cy = start_y + dy * steps;

        if cx < 0 || cx >= level.width as i32 || cy < 0 || cy >= level.height as i32 {
            break;
        }

        if level.cells[cy as usize * level.width + cx as usize] {
            hit_x = cx as f32 + 0.5 - (dx as f32 * 0.5);
            hit_y = cy as f32 + 0.5 - (dy as f32 * 0.5);
            break;
        }

        let mut hit_someone = false;
        for other in players.values() {
            if other.id != shooter.id && other.is_alive {
                let ox = other.x.floor() as i32;
                let oy = other.y.floor() as i32;
                if ox == cx && oy == cy {
                    hit_player_id = Some(other.id);
                    hit_x = other.x;
                    hit_y = other.y;
                    hit_someone = true;
                    break;
                }
            }
        }

        if hit_someone {
            break;
        }
        steps += 1;
    }

    lasers.push(LaserEffect {
        from_x: shooter.x,
        from_y: shooter.y,
        to_x: hit_x,
        to_y: hit_y,
        duration: 0.2,
    });

    if let Some(target_id) = hit_player_id {
        let mut target_died = false;
        let mut target_name = String::new();
        if let Some(target) = players.get_mut(&target_id) {
            target.health -= 35;
            target_name = target.name.clone();
            if target.health <= 0 {
                target.health = 0;
                target.is_alive = false;
                target.respawn_timer = 2.0;
                target.deaths += 1;
                target_died = true;
            }
        }
        if target_died {
            if let Some(s) = players.get_mut(&shooter_id) {
                s.score += 1;
                println!("*** {} KILLED {}! ***", s.name, target_name);
            }
        }
    }
}

pub fn reset_all_positions(players: &mut HashMap<u32, PlayerState>, level: &Level) {
    let mut occupied = HashSet::new();
    for player in players.values_mut() {
        let (sx, sy) = find_random_spawn(level, &occupied);
        player.x = sx as f32 + 0.5;
        player.y = sy as f32 + 0.5;
        player.health = 100;
        player.is_alive = true;
        occupied.insert((sx, sy));
    }
}

pub fn find_random_spawn(level: &Level, occupied: &HashSet<(usize, usize)>) -> (usize, usize) {
    let mut empty_cells = Vec::new();
    for r in 0..level.height {
        for c in 0..level.width {
            if !level.cells[r * level.width + c] && !occupied.contains(&(c, r)) {
                empty_cells.push((c, r));
            }
        }
    }
    if empty_cells.is_empty() {
        for r in 0..level.height {
            for c in 0..level.width {
                if !level.cells[r * level.width + c] {
                    return (c, r);
                }
            }
        }
        return (1, 1);
    }
    let mut rng = thread_rng();
    *empty_cells.choose(&mut rng).unwrap()
}
