use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    MoveForward,
    MoveBackward,
    TurnLeft,
    TurnRight,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Join { name: String },
    Input { action: PlayerAction },
    Shoot,
    RequestLevel { level_idx: usize },
    BackToLobby,
    Heartbeat,
    Leave,
    CustomMap { width: usize, height: usize, cells: Vec<bool> },
    StartGame,
    ToggleBots,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchState {
    Lobby,
    Playing,
    GameOver,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Welcome {
        player_id: u32,
        map_width: usize,
        map_height: usize,
        map_cells: Vec<bool>,
        level_index: usize,
        is_host: bool,
    },
    Tick(ServerStateTick),
    MapUpdate {
        map_width: usize,
        map_height: usize,
        map_cells: Vec<bool>,
        level_index: usize,
    },
    Reject {
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerState {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub dir_idx: usize,
    pub score: i32,
    pub deaths: i32,
    pub health: i32,
    pub is_alive: bool,
    pub is_bot: bool,
    pub respawn_timer: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LaserEffect {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub duration: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerStateTick {
    pub tick_id: u64,
    pub players: Vec<PlayerState>,
    pub lasers: Vec<LaserEffect>,
    pub level_index: usize,
    pub match_state: MatchState,
    pub round_time_left: f32,
    pub host_id: u32,
    pub bots_enabled: bool,
}
