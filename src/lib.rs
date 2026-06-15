pub mod protocol;
pub mod levels;
pub mod raycast;

pub use protocol::*;
pub use levels::*;
pub use raycast::*;

pub const DEFAULT_PORT: u16 = 10500;
pub const TICK_RATE_HZ: u64 = 30;
pub const HEARTBEAT_TIMEOUT_SECS: f32 = 5.0;
pub const MATCH_DURATION_SECS: f32 = 120.0;

pub const DIR_COORDS: [(i32, i32); 4] = [
    (1, 0),  // 0: East
    (0, 1),  // 1: South
    (-1, 0), // 2: West
    (0, -1), // 3: North
];

pub fn dir_idx_to_angle(dir_idx: usize) -> f32 {
    match dir_idx {
        0 => 0.0,
        1 => std::f32::consts::FRAC_PI_2,
        2 => std::f32::consts::PI,
        3 => -std::f32::consts::FRAC_PI_2,
        _ => 0.0,
    }
}
