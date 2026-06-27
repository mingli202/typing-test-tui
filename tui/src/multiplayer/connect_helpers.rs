use std::sync::{Arc, RwLock};

use futures::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_util::sync::CancellationToken;

use crate::typing::Typing;
use crate::util::toast::{self, ToastMessage};
use crate::{CustomEvent, ws_url};

use super::models::{LobbyInfo, NewGame, PlayersInfoSnapshot};
use super::{GameModel, GameStatus, Lobby};

/// Connects to the ws
pub async fn connect_to_ws(
    game_model: Arc<RwLock<GameModel>>,
    cancel_token: CancellationToken,
    event_tx: UnboundedSender<CustomEvent>,
    write_rx: UnboundedReceiver<String>,
) -> color_eyre::Result<()> {
    let request = ws_url().into_client_request()?;

    let (stream, _) = connect_async(request).await?;

    {
        let mut lock = game_model.write().unwrap();
        lock.is_connected = true;
    }

    let (write, read) = stream.split();

    let (read_tx, read_rx) = mpsc::unbounded_channel::<String>();

    init_write_task(write, write_rx, cancel_token.clone());
    init_read_task(read, read_tx, cancel_token.clone());
    init_recv_msg_task(game_model, read_rx, event_tx, cancel_token.clone());

    Ok(())
}

/// inits the task that will listen for messages to be sent to the websocket
fn init_write_task<T: SinkExt<Message> + Unpin + Send + 'static>(
    mut write: T,
    mut write_rx: UnboundedReceiver<String>,
    cancel_token: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = write_rx.recv() => {
                    let send_msg = Message::Text(Utf8Bytes::from(msg));
                    let res = write.send(send_msg).await;
                    if res.is_err() {
                        return;
                    }
                }
                _ = cancel_token.cancelled() => {
                    return;
                }
            }
        }
    });
}

/// inits the task that will listen for messages received from the websocket
fn init_read_task<E, T: Stream<Item = Result<Message, E>> + Unpin + Send + 'static>(
    mut read: T,
    read_tx: UnboundedSender<String>,
    cancel_token: CancellationToken,
) where
    E: std::error::Error,
{
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = match msg {
                                Ok(m) => m,
                                Err(_) => {
                                    break;
                                }
                            };

                            if !msg.is_text() {
                                continue;
                            }

                            let text = match msg.to_text() {
                                Ok(t) => t,
                                Err(_) => {
                                    break;
                                }
                            };

                            let res = read_tx.send(text.to_string());
                            if res.is_err() {
                                break;
                            }
                        },
                        None => {
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    return
                }
            }
        }

        cancel_token.cancel();
    });
}

/// inits the task that will listen for messages send through the given read channel
/// its to have a dedicated task for handling game_model state change
fn init_recv_msg_task(
    game_model: Arc<RwLock<GameModel>>,
    mut read_rx: UnboundedReceiver<String>,
    event_tx: UnboundedSender<CustomEvent>,
    cancel_token: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = read_rx.recv() => {
                    if let Err(err) = parse_ws_msg(&msg, Arc::clone(&game_model)) {
                        let res = toast::send(&event_tx, ToastMessage::error(err));
                        if res.is_err() {
                            return;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    return;
                }
            }
        }
    });
}

/// parses the msg into the commands and execute them
fn parse_ws_msg(msg: &str, game_model: Arc<RwLock<GameModel>>) -> Result<(), String> {
    let words: Vec<&str> = msg.split(" ").collect();

    if words.is_empty() || words[0].is_empty() {
        return Err("msg did not contain a cmd".to_string());
    }

    let cmd = words[0];

    match cmd {
        "LobbyInfo" => {
            let lobby_info = parse_payload_json::<LobbyInfo>(&words)?;
            let mut lock = game_model.write().unwrap();
            let lobby_id = lobby_info.lobby_id.clone();

            lock.lobby = Some(Lobby {
                typing: Typing::new(&lobby_info.data.text).stop_on_error(true),
                lobby_info,
                section_wpm: vec![],
                last_section_taken: (None, 0),
            });
            lock.active_lobby_id = Some(lobby_id.clone());
            lock.game_status = Some(GameStatus::Waiting);

            if lock.pending_lobby.lobby_id.as_deref() == Some(lobby_id.as_str()) {
                lock.pending_lobby.lobby_id = None;
            }

            if let Some(pending_players) = lock.pending_lobby.pending_players.take()
                && pending_players.lobby_id == lobby_id
            {
                lock.players_info = Some(pending_players);
            }
        }
        "NewGame" => {
            let new_game = parse_payload_json::<NewGame>(&words)?;
            let mut lock = game_model.write().unwrap();
            if let Some(lobby) = &mut lock.lobby {
                lobby.typing = Typing::new(&new_game.data.text).stop_on_error(true);
                lobby.lobby_info.data = new_game.data;
                lobby.section_wpm = vec![];
                lobby.last_section_taken = (None, 0);
            }
            lock.players_info = Some(new_game.players_info)
        }
        "EndGame" => {
            let player_info = parse_payload_json::<PlayersInfoSnapshot>(&words)?;
            let mut lock = game_model.write().unwrap();

            // Freeze local elapsed time when the game ends so post-game net WPM is stable
            // even if this client did not reach local typing completion.
            if let Some(lobby) = &mut lock.lobby {
                lobby.typing.end_now();
            }

            lock.players_info = Some(player_info);
            lock.game_status = Some(GameStatus::End);
        }
        "Error" => {
            let msg = get_payload_from_words(&words)?;
            return Err(msg);
        }
        "UserId" => {
            let user_id = get_payload_from_words(&words)?;
            let mut lock = game_model.write().unwrap();
            lock.user_id = Some(user_id);
        }
        "LeaveGroup" => {
            let did_succeed = parse_payload_json::<bool>(&words)?;

            if !did_succeed {
                return Err("Something went wrong leaving the group".to_string());
            }

            clear_game_model(game_model);
        }
        "PlayersInfo" => {
            let incoming_players = parse_payload_json::<PlayersInfoSnapshot>(&words)?;
            update_players(game_model, incoming_players);
        }
        "Countdown" => {
            let countdown = parse_payload_json::<i8>(&words)?;

            let mut lock = game_model.write().unwrap();
            lock.game_status = Some(GameStatus::Countdown(countdown));
        }
        "StartGame" => {
            let mut lock = game_model.write().unwrap();
            lock.game_status = Some(GameStatus::Playing);
            if let Some(ref mut lobby) = lock.lobby {
                lobby.typing.start();
            }
        }
        _ => return Err(format!("cmd {} unsupported", cmd)),
    };

    Ok(())
}

/// Returns the string after the cmd.
/// Returns an error if there is nothing after the first command
/// Assumes the shape of the words is <cmd> <...payload>, meaning everything after cmd is joined
/// into a singular string
fn get_payload_from_words(words: &[&str]) -> Result<String, String> {
    if words.len() < 2 {
        return Err("msg did not contain a payload".to_string());
    }

    let payload_str = words[1..].join(" ");

    Ok(payload_str)
}

/// Deserializes the payload into the given type
fn parse_payload_json<T: for<'a> Deserialize<'a>>(words: &[&str]) -> Result<T, String> {
    let payload = get_payload_from_words(words)?;

    serde_json::from_str::<T>(&payload).map_err(|err| err.to_string())
}

/// clear the game state when leaving
fn clear_game_model(game_model: Arc<RwLock<GameModel>>) {
    let mut lock = game_model.write().unwrap();

    lock.active_lobby_id = None;
    lock.pending_lobby.lobby_id = None;
    lock.pending_lobby.pending_players = None;
    lock.players_info = None;
    lock.lobby = None;
    lock.game_status = None;
}

/// updates the players if it's new
/// the incoming_playings will always be specific to the current group the user is in because the
/// backend will not allow the user to join another group if the user is already in a group.
fn update_players(game_model: Arc<RwLock<GameModel>>, incoming_players: PlayersInfoSnapshot) {
    let mut lock = game_model.write().unwrap();
    let expected_lobby_id = lock
        .active_lobby_id
        .as_deref()
        .or(lock.pending_lobby.lobby_id.as_deref());

    if expected_lobby_id != Some(incoming_players.lobby_id.as_str()) {
        lock.pending_lobby.pending_players = Some(incoming_players);
        return;
    }

    match &mut lock.players_info {
        Some(players) => {
            let is_newer_version = players.version < incoming_players.version;

            if is_newer_version {
                *players = incoming_players;
            }
        }
        None => lock.players_info = Some(incoming_players),
    }
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, VecDeque};
    use std::fmt::Display;
    use std::task::Poll;
    use std::time::{Duration, Instant};

    use crate::multiplayer::MultiplayerModel;
    use crate::util::data_provider::Data;

    use super::*;
    use futures::{Sink, StreamExt};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_get_payload_from_words() {
        let s = "cmd asdfasdf".split(" ").collect::<Vec<&str>>();

        let res = get_payload_from_words(&s);

        assert_eq!(res.is_ok(), true)
    }

    #[test]
    fn test_get_payload_from_words_no_payload() {
        let s = "cmd".split(" ").collect::<Vec<&str>>();

        let res = get_payload_from_words(&s);

        assert_eq!(res, Err("msg did not contain a payload".to_string()))
    }

    #[test]
    fn test_get_payload_json() {
        let json_str = json!({
            "lobby_id": "some-id",
            "data": {
                "source": "test source",
                "text": "test text"
            }
        })
        .to_string();

        let s = "cmd ".to_string() + &json_str;
        let s = s.split(" ").collect::<Vec<&str>>();

        let res = parse_payload_json::<LobbyInfo>(&s);

        assert_eq!(
            res,
            Ok(LobbyInfo {
                lobby_id: "some-id".to_string(),
                data: Data {
                    source: "test source".to_string(),
                    text: "test text".to_string()
                }
            })
        );
    }

    #[test]
    fn test_get_payload_json_wrong_format() {
        let json_str = json!({
            "lobby_id": "some-id",
            "data": {
                "source": "test source",
                "wrong format": 123
            }
        })
        .to_string();

        let s = "cmd ".to_string() + &json_str;
        let s = s.split(" ").collect::<Vec<&str>>();

        let res = parse_payload_json::<LobbyInfo>(&s);
        println!("res: {:?}", res);

        assert_eq!(res.is_err(), true)
    }

    #[test]
    fn test_parse_ws_msg_lobby_info() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));

        let json_str = json!({
            "lobby_id": "some-id",
            "data": {
                "source": "test source",
                "text": "test text"
            }
        })
        .to_string();

        let msg = "LobbyInfo ".to_string() + &json_str;

        assert_eq!(parse_ws_msg(&msg, Arc::clone(&game_model)), Ok(()));
        assert_eq!(
            game_model
                .read()
                .unwrap()
                .lobby
                .as_ref()
                .unwrap()
                .lobby_info,
            LobbyInfo {
                lobby_id: "some-id".to_string(),
                data: Data {
                    text: "test text".to_string(),
                    source: "test source".to_string()
                }
            }
        )
    }

    #[test]
    fn test_parse_ws_msg_user_id() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));

        assert_eq!(
            parse_ws_msg("UserId test-user-id", Arc::clone(&game_model)),
            Ok(())
        );
        assert_eq!(
            game_model.read().unwrap().user_id,
            Some("test-user-id".to_string())
        )
    }

    #[test]
    fn test_parse_ws_msg_leave_group_success_clears_lobby_state() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));

        let lobby_info = LobbyInfo {
            lobby_id: "lobby-1".to_string(),
            data: Data {
                text: "text".to_string(),
                source: "source".to_string(),
            },
        };
        let players_info = players_info_snapshot("lobby-1", 1, &["user-1"]);

        assert_eq!(
            parse_ws_msg("UserId user-1", Arc::clone(&game_model)),
            Ok(())
        );
        assert_eq!(
            parse_ws_msg(
                &format!("LobbyInfo {}", serde_json::to_string(&lobby_info).unwrap()),
                Arc::clone(&game_model)
            ),
            Ok(())
        );
        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&players_info).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );
        assert_eq!(parse_ws_msg("Countdown 3", Arc::clone(&game_model)), Ok(()));

        assert_eq!(
            parse_ws_msg("LeaveGroup true", Arc::clone(&game_model)),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.user_id, Some("user-1".to_string()));
        assert_eq!(lock.lobby.is_none(), true);
        assert_eq!(lock.players_info, None);
        assert!(lock.game_status.is_none());
    }

    #[test]
    fn test_parse_ws_msg_players_info_ignores_stale_snapshot_for_same_lobby() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        {
            let mut lock = game_model.write().unwrap();
            lock.pending_lobby.lobby_id = Some("lobby-1".to_string());
        }

        let fresh_players = players_info_snapshot("lobby-1", 2, &["new-player"]);
        let stale_players = players_info_snapshot("lobby-1", 1, &["old-player"]);

        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&fresh_players).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );
        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&stale_players).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.players_info, Some(fresh_players));
    }

    #[test]
    fn test_parse_ws_msg_players_info_is_accepted_before_lobby_info_if_join_is_pending() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        {
            let mut lock = game_model.write().unwrap();
            lock.pending_lobby.lobby_id = Some("target-lobby".to_string());
        }

        let players_info = players_info_snapshot("target-lobby", 1, &["user-1"]);
        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&players_info).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.players_info, Some(players_info));
        assert_eq!(
            lock.pending_lobby.lobby_id,
            Some("target-lobby".to_string())
        );
    }

    #[test]
    fn test_parse_ws_msg_players_info_is_updated_after_making_new_lobby_and_joining() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));

        let players_info = players_info_snapshot("target-lobby", 1, &["user-1"]);
        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&players_info).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lobby_info = LobbyInfo {
            data: Data {
                text: "some text".to_string(),
                source: "some source".to_string(),
            },
            lobby_id: "target-lobby".to_string(),
        };

        assert_eq!(
            parse_ws_msg(
                &format!("LobbyInfo {}", serde_json::to_string(&lobby_info).unwrap()),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.players_info, Some(players_info));
        assert_eq!(
            lock.lobby.as_ref().unwrap().lobby_info.lobby_id,
            "target-lobby".to_string()
        );
        assert_eq!(lock.pending_lobby.pending_players, None);
    }

    #[test]
    fn test_parse_ws_msg_players_info_is_rejected_for_wrong_lobby_when_join_is_pending() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        {
            let mut lock = game_model.write().unwrap();
            lock.pending_lobby.lobby_id = Some("target-lobby".to_string());
        }

        let wrong_lobby_players = players_info_snapshot("other-lobby", 99, &["user-1"]);
        assert_eq!(
            parse_ws_msg(
                &format!(
                    "PlayersInfo {}",
                    serde_json::to_string(&wrong_lobby_players).unwrap()
                ),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.players_info, None);
    }

    #[test]
    fn test_parse_ws_msg_lobby_info_sets_active_lobby_and_clears_matching_pending_join() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        {
            let mut lock = game_model.write().unwrap();
            lock.pending_lobby.lobby_id = Some("lobby-2".to_string());
        }

        let lobby_info = LobbyInfo {
            lobby_id: "lobby-2".to_string(),
            data: Data {
                text: "text".to_string(),
                source: "source".to_string(),
            },
        };

        assert_eq!(
            parse_ws_msg(
                &format!("LobbyInfo {}", serde_json::to_string(&lobby_info).unwrap()),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let lock = game_model.read().unwrap();
        assert_eq!(lock.active_lobby_id, Some("lobby-2".to_string()));
        assert_eq!(lock.pending_lobby.lobby_id, None);
    }

    fn players_info_snapshot(
        lobby_id: &str,
        version: u64,
        player_ids: &[&str],
    ) -> PlayersInfoSnapshot {
        let players = player_ids
            .iter()
            .map(|id| {
                (
                    (*id).to_string(),
                    super::super::models::PlayerInfo {
                        name: "Handsome Guest".to_string(),
                        is_leader: *id == player_ids[0],
                        wpm: 0.0,
                        progress_percent: 0,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        PlayersInfoSnapshot {
            lobby_id: lobby_id.to_string(),
            version,
            players,
        }
    }

    #[tokio::test]
    async fn test_end_game_freezes_local_net_wpm_for_post_game_graph() {
        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));

        let lobby_info = LobbyInfo {
            lobby_id: "lobby-1".to_string(),
            data: Data {
                text: "hello world".to_string(),
                source: "source".to_string(),
            },
        };

        assert_eq!(
            parse_ws_msg(
                &format!("LobbyInfo {}", serde_json::to_string(&lobby_info).unwrap()),
                Arc::clone(&game_model)
            ),
            Ok(())
        );
        assert_eq!(parse_ws_msg("StartGame", Arc::clone(&game_model)), Ok(()));

        {
            let mut lock = game_model.write().unwrap();
            let lobby = lock.lobby.as_mut().unwrap();
            for c in "hello".chars() {
                lobby.typing.on_type(c);
            }
        }

        tokio::time::sleep(Duration::from_millis(1200)).await;

        let final_players = players_info_snapshot("lobby-1", 2, &["user-1"]);
        assert_eq!(
            parse_ws_msg(
                &format!("EndGame {}", serde_json::to_string(&final_players).unwrap()),
                Arc::clone(&game_model)
            ),
            Ok(())
        );

        let wpm_after_end_1 = {
            let lock = game_model.read().unwrap();
            lock.lobby.as_ref().unwrap().typing.net_wpm()
        };

        tokio::time::sleep(Duration::from_millis(1200)).await;

        let wpm_after_end_2 = {
            let lock = game_model.read().unwrap();
            lock.lobby.as_ref().unwrap().typing.net_wpm()
        };

        assert_eq!(wpm_after_end_1, wpm_after_end_2);
    }

    #[derive(Debug, PartialEq, PartialOrd)]
    struct MockError {
        err: String,
    }

    impl Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.err)
        }
    }

    impl std::error::Error for MockError {}

    struct MockWebsocketStream {
        messages: VecDeque<Message>,
    }

    impl MockWebsocketStream {
        fn new(messages: Vec<String>) -> MockWebsocketStream {
            MockWebsocketStream {
                messages: messages
                    .into_iter()
                    .map(|msg| Message::Text(Utf8Bytes::from(&msg)))
                    .collect(),
            }
        }
    }

    impl Stream for MockWebsocketStream {
        type Item = Result<Message, MockError>;
        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let msg = self.get_mut().messages.pop_front();
            Poll::Ready(msg.map(Ok))
        }
    }

    impl Sink<Message> for MockWebsocketStream {
        type Error = MockError;

        fn poll_ready(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            self.get_mut().messages.push_back(item);
            Ok(())
        }
        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            self.poll_flush(cx)
        }
    }

    #[tokio::test]
    async fn test_init_write_task() {
        // Arrange
        let s = MockWebsocketStream::new(vec![
            "NewGroup".to_string(),
            "JoinGroup asdfgh".to_string(),
            "LeaveGroup".to_string(),
        ]);
        let (write, mut read) = s.split();

        let (write_tx, write_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        let model = MultiplayerModel {
            cancel_token: CancellationToken::new(),
            game_model,
            write_tx,
            input_lobby_id: vec![],
            last_sent_update: Instant::now(),
            is_focused: true,
        };

        // Act
        init_write_task(write, write_rx, model.cancel_token.clone());

        // Assert
        assert_eq!(
            read.next().await,
            Some(Ok(Message::Text(Utf8Bytes::from("NewGroup"))))
        );
        assert_eq!(
            read.next().await,
            Some(Ok(Message::Text(Utf8Bytes::from("JoinGroup asdfgh"))))
        );
        assert_eq!(
            read.next().await,
            Some(Ok(Message::Text(Utf8Bytes::from("LeaveGroup"))))
        );
        assert_eq!(read.next().await, None);

        // Cleanup
        model.cancel_token.cancel();
    }

    #[tokio::test]
    async fn test_init_read_task() {
        // Arrange
        let s = MockWebsocketStream::new(vec![
            "NewGroup".to_string(),
            "JoinGroup asdfgh".to_string(),
            "LeaveGroup".to_string(),
        ]);
        let (_, read) = s.split();

        let (read_tx, mut read_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let cancel_token = CancellationToken::new();

        // Act
        init_read_task(read, read_tx, cancel_token.clone());

        // Assert
        assert_eq!(read_rx.recv().await, Some("NewGroup".to_string()));
        assert_eq!(read_rx.recv().await, Some("JoinGroup asdfgh".to_string()));
        assert_eq!(read_rx.recv().await, Some("LeaveGroup".to_string()));

        // Cleanup
        cancel_token.cancel();
    }

    #[tokio::test]
    async fn test_init_recv_msg_task() {
        // Arrange
        let (read_tx, read_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (write_tx, _) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (event_tx, _) = tokio::sync::mpsc::unbounded_channel::<CustomEvent>();

        let game_model: Arc<RwLock<GameModel>> = Arc::new(RwLock::new(GameModel::default()));
        let model = MultiplayerModel {
            cancel_token: CancellationToken::new(),
            game_model: Arc::clone(&game_model),
            write_tx,
            input_lobby_id: vec![],
            last_sent_update: Instant::now(),
            is_focused: true,
        };

        let lobby_info = LobbyInfo {
            lobby_id: "asdfgh".to_string(),
            data: Data {
                text: "text".to_string(),
                source: "source".to_string(),
            },
        };

        // Act
        init_recv_msg_task(
            Arc::clone(&model.game_model),
            read_rx,
            event_tx,
            model.cancel_token.clone(),
        );

        let msg1 = "LobbyInfo ".to_string() + &serde_json::to_string(&lobby_info).unwrap();

        let _ = read_tx.send(msg1);
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Assert
        {
            let lock = game_model.read().unwrap();
            assert_eq!(lock.lobby.as_ref().unwrap().lobby_info, lobby_info);
        }

        // Cleanup
        model.cancel_token.cancel();
    }
}
