use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Clone)]
pub struct Level {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<bool>,
}

pub fn get_static_level(idx: usize) -> Level {
    match idx {
        0 => parse_map_str(
            12,
            12,
            "############\n\
             #          #\n\
             # ##  ##   #\n\
             #          #\n\
             #   ####   #\n\
             #   ####   #\n\
             #          #\n\
             #  ##  ##  #\n\
             #          #\n\
             #   ####   #\n\
             #          #\n\
             ############",
        ),
        1 => parse_map_str(
            16,
            16,
            "################\n\
             #              #\n\
             #  #####  ###  #\n\
             #  #   #    #  #\n\
             #  # # #  # #  #\n\
             #  # # #  # #  #\n\
             #  ###    ###  #\n\
             #              #\n\
             #  ###    ###  #\n\
             #  # #    # #  #\n\
             #  # #    # #  #\n\
             #  # #    # #  #\n\
             #  ###    ###  #\n\
             #              #\n\
             #              #\n\
             ################",
        ),
        _ => parse_map_str(
            21,
            21,
            "#####################\n\
             #                   #\n\
             # ######### ####### #\n\
             # #       # #     # #\n\
             # # ##### # # ### # #\n\
             # # #   # # # # # # #\n\
             # # ### # # # ### # #\n\
             # #   #   # #     # #\n\
             # ### ##### ####### #\n\
             #   # #             #\n\
             ### # # ######### ###\n\
             #   # # #       # #\n\
             # ### # # ##### # #\n\
             # #   # # #   # # #\n\
             # # ### # ### # # #\n\
             # # #   #     # # #\n\
             # # # ######### # #\n\
             # # #             #\n\
             # # ###############\n\
             #                  #\n\
             #####################",
        ),
    }
}

fn parse_map_str(width: usize, height: usize, map_str: &str) -> Level {
    let mut cells = vec![false; width * height];
    let mut r = 0;
    for line in map_str.lines() {
        if r >= height {
            break;
        }
        let chars: Vec<char> = line.chars().collect();
        for c in 0..width {
            if c < chars.len() {
                cells[r * width + c] = chars[c] == '#';
            } else {
                cells[r * width + c] = true;
            }
        }
        r += 1;
    }
    Level { width, height, cells }
}

pub fn generate_random_maze(width: usize, height: usize) -> Level {
    let w = if width % 2 == 0 { width + 1 } else { width };
    let h = if height % 2 == 0 { height + 1 } else { height };

    let mut cells = vec![true; w * h];
    let mut stack = Vec::new();

    cells[1 * w + 1] = false;
    stack.push((1, 1));

    let mut rng = thread_rng();

    while let Some((cx, cy)) = stack.last().cloned() {
        let mut neighbors = Vec::new();
        let dirs = [(-2, 0), (2, 0), (0, -2), (0, 2)];

        for (dx, dy) in dirs {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if nx > 0 && nx < (w - 1) as i32 && ny > 0 && ny < (h - 1) as i32 {
                let ux = nx as usize;
                let uy = ny as usize;
                if cells[uy * w + ux] {
                    neighbors.push((ux, uy));
                }
            }
        }

        if neighbors.is_empty() {
            stack.pop();
        } else {
            let &(nx, ny) = neighbors.choose(&mut rng).unwrap();
            let mx = (cx + nx) / 2;
            let my = (cy + ny) / 2;
            cells[my * w + mx] = false;
            cells[ny * w + nx] = false;
            stack.push((nx, ny));
        }
    }

    Level { width: w, height: h, cells }
}
