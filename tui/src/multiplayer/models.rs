use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::util::data_provider::Data;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub is_leader: bool,
    pub wpm: f64,
    pub progress_percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfoSnapshot {
    pub lobby_id: String,
    pub version: u64,
    pub players: HashMap<String, PlayerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub lobby_id: String,
    pub data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewGame {
    pub data: Data,
    pub players_info: PlayerInfoSnapshot,
}
