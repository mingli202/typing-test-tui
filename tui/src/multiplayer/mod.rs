use std::sync::{Arc, RwLock};

use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::CustomEvent;
use crate::util::toast::{self, ToastMessage};

use self::models::{LobbyInfo, NewGame, PlayerInfoSnapshot};

mod models;

pub struct SharedModel {
    user_id: String,
    player_info: PlayerInfoSnapshot,
    lobby_info: LobbyInfo,
}

pub struct MultiplayerModel {
    share_model: Arc<RwLock<SharedModel>>,
    write_tx: UnboundedSender<String>,
    read_rx: UnboundedReceiver<String>,
}

// Connects to the ws
pub async fn connect_to_ws(event_tx: UnboundedSender<CustomEvent>) {
    let request = "ws://localhost:8080/ws".into_client_request().unwrap();

    let (stream, _) = connect_async(request).await.unwrap();

    let (write, mut read) = stream.split();

    let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();
    let (read_tx, read_rx) = mpsc::unbounded_channel::<String>();

    let handle: JoinHandle<color_eyre::Result<()>> = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            let msg = msg?;

            if !msg.is_text() {
                return Ok(());
            }

            let text = msg.to_text()?;
        }

        Ok(())
    });
}

// parses the msg into the commands and execute them
fn parse_ws_msg(
    msg: &str,
    shared_model: Arc<RwLock<SharedModel>>,
    event_tx: UnboundedSender<CustomEvent>,
) {
    let words: Vec<&str> = msg.split(" ").collect();

    if words.is_empty() {
        let _ = toast::send(
            &event_tx,
            ToastMessage::error("msg did not contain a cmd".to_string()),
        );
        return;
    }

    let cmd = words[0];

    match cmd {
        "LobbyInfo" => match parse_payload_str::<LobbyInfo>(&words) {
            Ok(lobby_info) => {
                let mut lock = shared_model.write().unwrap();
                lock.lobby_info = lobby_info;
            }
            Err(err) => {
                let _ = toast::send(&event_tx, ToastMessage::error(err));
            }
        },
        "NewGame" => match parse_payload_str::<NewGame>(&words) {
            Ok(new_game) => {
                let mut lock = shared_model.write().unwrap();
                lock.player_info = new_game.players_info;
                lock.lobby_info.data = new_game.data
            }
            Err(err) => {
                let _ = toast::send(&event_tx, ToastMessage::error(err));
            }
        },
        "EndGame" => match parse_payload_str::<PlayerInfoSnapshot>(&words) {
            Ok(player_info) => {
                let mut lock = shared_model.write().unwrap();
                lock.player_info = player_info;
            }
            Err(err) => {
                let _ = toast::send(&event_tx, ToastMessage::error(err));
            }
        },
        "Error" => {}
        "UserId" => {}
        "PlayersInfo" => match parse_payload_str::<PlayerInfoSnapshot>(&words) {
            Ok(player_info) => {
                let mut lock = shared_model.write().unwrap();
                lock.player_info = player_info;
            }
            Err(err) => {
                let _ = toast::send(&event_tx, ToastMessage::error(err));
            }
        },
        "Countdown" => {}
        _ => {}
    }
}

fn parse_payload_str<T: for<'a> Deserialize<'a>>(words: &[&str]) -> Result<T, String> {
    if words.len() < 2 {
        return Err("msg did not contain a payload".to_string());
    }

    let payload_str = words[1..].join(" ");
    serde_json::from_str::<T>(&payload_str).map_err(|err| err.to_string())
}
