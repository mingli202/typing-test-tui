use self::models::PlayerInfoSnapshot;

mod models;

pub struct SharedModel {}

pub struct MultiplayerModel {
    playerInfo: PlayerInfoSnapshot,
}
