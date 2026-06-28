use std::collections::HashMap;
use rand::thread_rng;
use rand::Rng;
use mp::*;

use crate::physics::{cell_occupied_by_other, check_line_of_sight, fire_laser};

pub fn update_bots(
    bot_ids: &[u32],
    players: &mut HashMap<u32, PlayerState>,
    level: &Level,
    lasers: &mut Vec<LaserEffect>,
    bot_cooldowns: &mut HashMap<u32, u32>,
) {
    let mut rng = thread_rng();
    for &bot_id in bot_ids {
        let mut bot = match players.get(&bot_id) {
            Some(p) if p.is_alive => p.clone(),
            _ => continue,
        };

        let cooldown = bot_cooldowns.entry(bot_id).or_insert(0);
        if *cooldown > 0 {
            *cooldown -= 1;
            continue;
        }

        let mut acted = false;
        let (target_id, _) = check_line_of_sight(bot.id, bot.x, bot.y, bot.dir_idx, players, level);
        
        if target_id.is_some() {
            fire_laser(bot.id, players, level, lasers);
            *cooldown = rng.gen_range(10..20);
            acted = true;
        } else {
            for dir in 0..4 {
                if dir == bot.dir_idx {
                    continue;
                }
                let (tid, dist) = check_line_of_sight(bot.id, bot.x, bot.y, dir, players, level);
                if tid.is_some() && dist < 6.0 {
                    bot.dir_idx = dir;
                    players.insert(bot.id, bot.clone());
                    *cooldown = rng.gen_range(5..12);
                    acted = true;
                    break;
                }
            }
        }

        if !acted {
            if rng.gen_range(0.0..1.0) < 0.65 {
                let (dx, dy) = DIR_COORDS[bot.dir_idx];
                let nx = bot.x.floor() as i32 + dx;
                let ny = bot.y.floor() as i32 + dy;
                if nx >= 0 && nx < level.width as i32 && ny >= 0 && ny < level.height as i32 {
                    if !level.cells[ny as usize * level.width + nx as usize]
                        && !cell_occupied_by_other(players, bot.id, nx, ny)
                    {
                        bot.x = nx as f32 + 0.5;
                        bot.y = ny as f32 + 0.5;
                    } else if rng.gen_bool(0.5) {
                        bot.dir_idx = (bot.dir_idx + 1) % 4;
                    } else {
                        bot.dir_idx = (bot.dir_idx + 3) % 4;
                    }
                }
            } else if rng.gen_bool(0.5) {
                bot.dir_idx = (bot.dir_idx + 1) % 4;
            } else {
                bot.dir_idx = (bot.dir_idx + 3) % 4;
            }
            players.insert(bot.id, bot);
            *cooldown = rng.gen_range(6..15);
        }
    }
}
