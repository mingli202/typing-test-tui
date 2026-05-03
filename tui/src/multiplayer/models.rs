use std::collections::HashMap;

use crate::util::data_provider::Data;

pub struct PlayerInfo {
    is_leader: bool,
    wpm: f64,
    progress_percent: u8,
}

pub struct PlayerInfoSnapshot {
    lobby_id: String,
    version: u64,
    players: HashMap<String, PlayerInfo>,
}

pub struct LobbyInfo {
    lobby_id: String,
    data: Data,
}

pub struct NewGame {
    data: Data,
    players_info: PlayerInfoSnapshot,
}
