use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyModifiers};
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, HorizontalAlignment, Layout, Offset, Rect, Size};
use ratatui::macros::{line, span};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::{self, Marker};
use ratatui::text::ToSpan;
use ratatui::widgets::{
    Axis, Block, Chart, Dataset, GraphType, LineGauge, Paragraph, Widget, Wrap,
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_util::sync::CancellationToken;

use crate::CustomEvent;
use crate::msg::Msg;
use crate::typing::{Typing, view_typing_test};
use crate::util::toast::ToastMessage;
use crate::util::{toast, view_helpers};

use self::connect_helpers::connect_to_ws;
use self::models::{LobbyInfo, PlayerInfo, PlayersInfoSnapshot, WsMsg};

mod connect_helpers;
mod models;

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
pub enum GameStatus {
    Waiting,
    Countdown(i8),
    Playing,
    Done,
    End,
}

pub struct Lobby {
    lobby_info: LobbyInfo,
    typing: Typing,

    section_wpm: Vec<(f64, f64)>,
    /// the time the last section was taken, and the number of characters typed at that point in time
    last_section_taken: (Option<Instant>, usize),
}

#[derive(Default)]
pub struct PendingLobby {
    lobby_id: Option<String>,
    pending_players: Option<PlayersInfoSnapshot>,
}

#[derive(Default)]
pub struct GameModel {
    is_connected: bool,
    user_id: Option<String>,
    active_lobby_id: Option<String>,
    pending_lobby: PendingLobby,
    players_info: Option<PlayersInfoSnapshot>,
    lobby: Option<Lobby>,
    game_status: Option<GameStatus>,
}

pub struct MultiplayerModel {
    game_model: Arc<RwLock<GameModel>>,
    write_tx: UnboundedSender<String>,
    input_lobby_id: Vec<char>,
    last_sent_update: Instant,
    is_focused: bool,

    cancel_token: CancellationToken,
}

impl MultiplayerModel {
    pub fn new(event_tx: UnboundedSender<CustomEvent>) -> Self {
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        let model = MultiplayerModel {
            game_model: Arc::new(RwLock::new(GameModel::default())),
            write_tx,
            input_lobby_id: vec![],
            last_sent_update: Instant::now(),
            is_focused: true,
            cancel_token: CancellationToken::new(),
        };

        let game_model = Arc::clone(&model.game_model);
        let cancel_token = model.cancel_token.clone();
        tokio::spawn(async move {
            if let Err(err) =
                connect_to_ws(game_model, cancel_token.clone(), event_tx.clone(), write_rx).await
            {
                let _ = toast::send(&event_tx, ToastMessage::error(err.to_string()));
                cancel_token.cancel();
            }
        });

        model
    }

    /// Sends the given message to the websocket
    pub fn send_msg(&self, msg: WsMsg) {
        let pending_join_lobby_id = match &msg {
            WsMsg::JoinGroup(group_id) => Some(group_id.clone()),
            _ => None,
        };
        let did_send = self.write_tx.send(msg.to_string()).is_ok();

        if did_send && let Some(group_id) = pending_join_lobby_id {
            let mut lock = self.game_model.write().unwrap();
            lock.pending_lobby.lobby_id = Some(group_id);
        }
    }
}

impl Drop for MultiplayerModel {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub fn update(
    model: &mut MultiplayerModel,
    event_tx: &UnboundedSender<CustomEvent>,
    msg: Msg,
) -> Option<crate::action::Action> {
    let is_in_lobby = {
        let lock = model.game_model.read().unwrap();
        lock.lobby.is_some()
    };

    if model.cancel_token.is_cancelled() {
        let _ = toast::send(
            event_tx,
            ToastMessage::error("Multiplayer crashed, back to singleplayer".to_string()),
        );
        return Some(crate::action::Action::SwitchToSinglePlayer);
    }

    match msg {
        Msg::FocusGained => model.is_focused = true,
        Msg::FocusLost => model.is_focused = false,
        _ => {}
    }

    if is_in_lobby {
        update_lobby_info(model, msg)
    } else {
        update_no_lobby_info(model, msg)
    }
}

/// the update part of the view without a lobby
fn update_no_lobby_info(model: &mut MultiplayerModel, msg: Msg) -> Option<crate::action::Action> {
    match msg {
        Msg::Key(key) => {
            match key.code {
                KeyCode::Char(c) => {
                    if c == 'n' && matches!(key.modifiers, KeyModifiers::CONTROL) {
                        model.send_msg(WsMsg::NewGroup);
                        return None;
                    }

                    if !c.is_ascii_lowercase() || model.input_lobby_id.len() >= 6 {
                        return None;
                    }

                    model.input_lobby_id.push(c);
                }
                KeyCode::Backspace => match key.modifiers {
                    KeyModifiers::CONTROL | KeyModifiers::ALT => {
                        model.input_lobby_id.clear();
                    }
                    _ => {
                        model.input_lobby_id.pop();
                    }
                },
                KeyCode::Enter => {
                    let lobby_id = model.input_lobby_id.iter().collect();
                    model.send_msg(WsMsg::JoinGroup(lobby_id));
                }
                _ => {}
            };
        }
        Msg::Tick => {}
        _ => {}
    };

    None
}

/// the update part of the view with a lobby
fn update_lobby_info(model: &mut MultiplayerModel, msg: Msg) -> Option<crate::action::Action> {
    match msg {
        Msg::Key(key) => match key.code {
            KeyCode::Char(c) => {
                let is_playing = {
                    let lock = model.game_model.read().unwrap();
                    lock.game_status == Some(GameStatus::Playing)
                };

                if is_playing {
                    let mut lock = model.game_model.write().unwrap();
                    if let Some(lobby) = &mut lock.lobby {
                        let done = lobby.typing.on_type(c);

                        let section = typing_progress(lobby) / 10;

                        if section > 0
                            && lobby.section_wpm.len() < section
                            && let Some(elapsed) = lobby.typing.elapsed_since_start_sec()
                        {
                            let elapsed_since_last_section =
                                lobby.last_section_taken.0.map_or(elapsed, |t| t.elapsed());

                            let elapsed_secs = elapsed_since_last_section.as_secs_f64();

                            if elapsed_secs > 0.0 {
                                let letters_typed = lobby.typing.letters_typed();
                                let letters_typed_since_last_section =
                                    letters_typed.saturating_sub(lobby.last_section_taken.1);

                                let wpm = 60.0 * (letters_typed_since_last_section as f64 / 5.0)
                                    / elapsed_secs;

                                lobby.section_wpm.push(((section * 10) as f64, wpm));
                                lobby.last_section_taken = (Some(Instant::now()), letters_typed);
                            }
                        }

                        if done {
                            send_update_stats(model, lobby);
                            lock.game_status = Some(GameStatus::Done);
                        }
                    }
                }
            }
            KeyCode::Esc => {
                model.send_msg(WsMsg::LeaveGroup);
            }
            KeyCode::Backspace => {
                let is_playing = {
                    let lock = model.game_model.read().unwrap();
                    lock.game_status == Some(GameStatus::Playing)
                };

                if is_playing {
                    let mut lock = model.game_model.write().unwrap();
                    if let Some(lobby) = &mut lock.lobby {
                        match key.modifiers {
                            KeyModifiers::CONTROL | KeyModifiers::ALT => {
                                lobby.typing.on_word_backspace();
                            }
                            _ => {
                                lobby.typing.on_backspace();
                            }
                        }
                    }
                }
            }
            KeyCode::Enter => {
                let can_start = {
                    let lock = model.game_model.read().unwrap();

                    let is_leader = if let Some(ref players) = lock.players_info
                        && let Some(ref user_id) = lock.user_id
                    {
                        players
                            .players
                            .get(user_id)
                            .is_some_and(|player| player.is_leader)
                    } else {
                        false
                    };

                    let is_waiting = if let Some(ref game_status) = lock.game_status
                        && (*game_status == GameStatus::Waiting || *game_status == GameStatus::End)
                    {
                        true
                    } else {
                        false
                    };

                    is_leader && is_waiting
                };

                if can_start {
                    model.send_msg(WsMsg::StartGame);
                }
            }
            _ => {}
        },
        Msg::Tick => {
            if model.last_sent_update.elapsed() > Duration::from_millis(200) {
                let lock = model.game_model.read().unwrap();

                if lock.game_status == Some(GameStatus::Playing)
                    && let Some(ref lobby) = lock.lobby
                {
                    send_update_stats(model, lobby);
                }

                model.last_sent_update = Instant::now();
            }
        }
        _ => {}
    };

    None
}

/// Sends the user's stats to the server
fn send_update_stats(model: &MultiplayerModel, lobby: &Lobby) {
    let wpm = lobby.typing.net_wpm();
    let progress = typing_progress(lobby);

    model.send_msg(WsMsg::UpdateStats(wpm, progress as u8));
}

/// Get the progress
fn typing_progress(lobby: &Lobby) -> usize {
    let total_len = lobby.lobby_info.data.text.len();

    if total_len == 0 {
        return 100;
    }

    let progress = lobby.typing.letters_typed() * 100 / total_len;

    if progress >= 100 {
        if let Some(word) = lobby.typing.get_curr_word()
            && word.is_error()
        {
            99
        } else {
            100
        }
    } else {
        progress
    }
}

pub fn view(model: &MultiplayerModel, area: Rect, buf: &mut Buffer) {
    let lock = model.game_model.read().unwrap();

    if !lock.is_connected {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |s| s.as_secs())
            % 4;

        let n_dots = ".".repeat(now as usize);

        let text = line!("Connecting", n_dots);
        let centered = area.centered(
            Constraint::Length(text.width() as u16),
            Constraint::Length(1),
        );
        text.render(centered, buf);

        view_helpers::view_bottom_menu(&["Singleplayer <C-p>"], area, buf);
        return;
    }

    match lock.lobby {
        None => {
            let lobby_text: String = model.input_lobby_id.iter().collect();
            let cursor = span![" "].bg(if model.is_focused {
                Color::White
            } else {
                Color::Reset
            });

            let line = line![lobby_text, cursor];
            let typing_box = Paragraph::new(line).block(Block::bordered().title("Lobby Id"));
            let enter_text = "Enter".to_span();
            let create_text = "Create Lobby <C-n>".to_span();

            let centered = area.centered(
                Constraint::Length(10 + enter_text.width() as u16 + enter_text.width() as u16),
                Constraint::Length(3),
            );

            let layout = Layout::new(
                Direction::Horizontal,
                vec![
                    Constraint::Length(10 + enter_text.width() as u16),
                    Constraint::Length(enter_text.width() as u16),
                ],
            )
            .spacing(2)
            .split(centered);

            typing_box.render(layout[0], buf);
            enter_text.render(layout[1].centered_vertically(Constraint::Length(1)), buf);

            let create_text_area = area
                .centered(
                    Constraint::Length(create_text.width() as u16),
                    Constraint::Length(1),
                )
                .offset(Offset {
                    x: 0,
                    y: layout[0].height as i32,
                });
            create_text.render(create_text_area, buf);

            view_helpers::view_bottom_menu(&["Singleplayer <C-p>"], area, buf);
        }
        Some(ref lobby) => {
            let lobby_line = line!("Id: ", span!(lobby.lobby_info.lobby_id));
            let lobby_line_area =
                area.centered_horizontally(Constraint::Length(lobby_line.width() as u16));
            lobby_line.render(lobby_line_area, buf);

            let is_leader = {
                if let Some(ref players) = lock.players_info
                    && let Some(ref user_id) = lock.user_id
                {
                    players
                        .players
                        .get(user_id)
                        .is_some_and(|player| player.is_leader)
                } else {
                    false
                }
            };

            let data_area = area.centered(Constraint::Max(80), Constraint::Length(3));
            let mut status_area = data_area.offset(Offset { x: 0, y: -2 });
            if let Some(ref game_status) = lock.game_status {
                status_area = view_game_status(game_status, is_leader, status_area, buf);
            }

            let max_offset = status_area.y.saturating_sub(1);
            if let Some(ref players) = lock.players_info
                && let Some(ref user_id) = lock.user_id
            {
                view_players(players, user_id, max_offset, area, buf);
            }

            if let Some(ref game_status) = lock.game_status
                && (*game_status == GameStatus::Done || *game_status == GameStatus::End)
            {
                let data_source =
                    Paragraph::new(format!("Source: {}", &lobby.lobby_info.data.source))
                        .wrap(Wrap { trim: true });
                let data_source_line_count = data_source.line_count(data_area.width);
                let mut data_source_area = data_area.resize(Size {
                    width: data_area.width,
                    height: data_source_line_count as u16,
                });
                data_source_area.y = area.bottom().saturating_sub(2);

                let data_source_area = data_source_area.offset(Offset {
                    x: 0,
                    y: -(data_source_line_count as i32),
                });
                data_source.render(data_source_area, buf);

                let graph_area_height = area
                    .height
                    .saturating_sub(data_area.y)
                    .saturating_sub(3)
                    .saturating_sub(data_source_area.height);
                let graph_area = data_area.resize(Size {
                    width: data_area.width,
                    height: graph_area_height,
                });
                let wpm = lobby.typing.net_wpm();
                view_section_wpm(&lobby.section_wpm, wpm, graph_area, buf);
            } else {
                view_typing_test(&lobby.typing, model.is_focused, data_area, buf);
            }

            view_helpers::view_bottom_menu(&["Singleplayer <C-p>  Leave <Esc>"], area, buf);
        }
    };
}

/// Show the game status
fn view_game_status(
    game_status: &GameStatus,
    is_leader: bool,
    area: Rect,
    buf: &mut Buffer,
) -> Rect {
    let txt = match game_status {
        GameStatus::Done => line!("Waiting for others to finish..."),
        GameStatus::Waiting => {
            if is_leader {
                line!("Press ENTER to start")
            } else {
                line!("Waiting for leader")
            }
        }
        GameStatus::Countdown(count_down) => line!("Start in ", count_down.to_span()),
        GameStatus::Playing => line!("Go!"),
        GameStatus::End => {
            if is_leader {
                line!("Game has ended, press ENTER to restart")
            } else {
                line!("Game has ended, wait for leader to start again or leave")
            }
        }
    };

    let p = Paragraph::new(txt).wrap(Wrap { trim: true }).centered();
    let n_line = p.line_count(area.width);

    let area = area.offset(Offset {
        x: 0,
        y: -(n_line as i32 - 1),
    });

    p.render(area, buf);

    area
}

/// renders the players
fn view_players(
    players_info: &PlayersInfoSnapshot,
    me: &str,
    max_offset: u16,
    area: Rect,
    buf: &mut Buffer,
) {
    let area = area
        .centered_horizontally(Constraint::Max(80))
        .offset(Offset { x: 0, y: 2 });

    let mut area = area.resize(Size {
        width: area.width,
        height: (area.height / 2).saturating_sub(6),
    });

    if area.height == 0 {
        return;
    }

    let players = players_info
        .players
        .iter()
        .filter(|(id, _)| *id != me)
        .sorted_by_key(|(id, player)| (player.progress_percent, *id))
        .rev();

    if let Some(me_info) = players_info.players.get(me) {
        view_player(me_info, true, area, buf);
        area.y += 1;
    }

    for (i, (_, player)) in players.enumerate() {
        let area = area.offset(Offset { x: 0, y: i as i32 });

        if area.y > max_offset {
            break;
        };

        view_player(player, false, area, buf);
    }
}

/// Render a player with their name, wpm, and progess bar
fn view_player(player: &PlayerInfo, is_me: bool, area: Rect, buf: &mut Buffer) {
    let ratio = player.progress_percent as f64 / 100.0;

    let wpm = player.wpm;
    let wpm_string = if wpm >= 100.0 {
        format!("{:.0}", wpm)
    } else if wpm >= 10.0 {
        format!("{:.1}", wpm)
    } else {
        format!("{:.2}", wpm)
    };

    let mut label = span!(format!("{} {}", player.name, wpm_string));

    if is_me {
        label = label.underlined();
    }

    LineGauge::default()
        .label(label)
        .filled_style(Style::new().white())
        .filled_symbol(symbols::line::THICK_HORIZONTAL)
        .unfilled_symbol(" ")
        .ratio(ratio.clamp(0.0, 1.0))
        .render(area, buf)
}

/// Renders the wpm per section when the user is done with the test
fn view_section_wpm(section_wpm: &[(f64, f64)], net_wpm: f64, area: Rect, buf: &mut Buffer) {
    let net_wpm_line = [(0.0, net_wpm), (100.0, net_wpm)];

    let datasets = vec![
        Dataset::default()
            .graph_type(GraphType::Line)
            .style(Style::default().dark_gray())
            .marker(Marker::Braille)
            .data(&net_wpm_line),
        Dataset::default()
            .graph_type(GraphType::Bar)
            .style(Style::default().white())
            .marker(Marker::HalfBlock)
            .data(section_wpm),
    ];

    // Create the X axis and define its properties
    let x_axis = Axis::default()
        .title("Section (%)")
        .style(Style::default().white())
        .bounds([0.0, 100.0])
        .labels(["0", "50", "100"]);

    let max_wpm = section_wpm
        .iter()
        .map(|(_, wpm)| wpm.ceil() as i32)
        .max()
        .unwrap_or(0)
        .max(net_wpm.ceil() as i32);

    // Make the graph go to 1 if it's less for prettier graph
    let max_wpm = if max_wpm <= 1 { 1 } else { max_wpm };

    // Create the Y axis and define its properties
    let y_axis = Axis::default()
        .title("WPM")
        .style(Style::default().white())
        .bounds([0.0, max_wpm as f64])
        .labels([
            "0.0".to_string(),
            format!("{}", max_wpm / 2),
            format!("{}", max_wpm),
        ]);

    // Create the chart and link all the parts together
    let chart = Chart::new(datasets)
        .block(
            Block::new()
                .title(format!("WPM per section ({:.1} net WPM)", net_wpm))
                .title_alignment(HorizontalAlignment::Center),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    chart.render(area, buf);
}
