use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::util::data_provider::Data;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    is_leader: bool,
    wpm: f64,
    progress_percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfoSnapshot {
    lobby_id: String,
    version: u64,
    players: HashMap<String, PlayerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LobbyInfo {
    lobby_id: String,
    data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewGame {
    data: Data,
    players_info: PlayerInfoSnapshot,
}
