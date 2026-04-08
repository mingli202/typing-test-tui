use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::data::Data;
use crate::message::Message;
use crate::toast::ToastMessage;
use crate::typing_test::TypingTest;
use crate::typing_test::mode_selection::ModeSelection;

pub struct AppModel {
    exit: bool,
    toast: Toast,
    config: Config,
    // (time, wpm)
    history: Vec<(f64, f64)>,
    mode: Mode,
    data: Data,
    screen: Screen,
}

struct StateModel {}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    Quote,
    Words(usize),
    Time(usize),
}

impl Mode {
    pub fn get_data(&self) -> Data {
        match self {
            Mode::Quote => Data::get_random_quote(),
            Mode::Words(n) => Data::get_n_random_words(*n),
            // TODO: new lines as the user reaches the end
            // max 80 char per line -> ~16 words
            // preload 4 lines
            //
            // NOTE: require refactor of current architecture or it will become messy
            // for now, just assume the user won't type more than 240 wpm
            Mode::Time(t) => {
                let mut data = Data::get_n_random_words(t * 4);
                data.source = format!("{} seconds", t);
                data
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TypingStats {
    wpm: f64,
    current_index: usize,
    elapsed: Duration,
}

pub enum Screen {
    TypingTest {
        typing_test: TypingTest,
        stats_last_updated_time: Instant,
        stats: TypingStats,
        selected_mode: ModeSelection,
    },
    EndScreen {
        wpm: f64,
        accuracy: usize,
    },
}

impl Screen {
    /// Gets a fresh typing test
    pub fn new_typing_test(text: &str, mode: Mode) -> Self {
        Screen::TypingTest {
            typing_test: TypingTest::new(text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
            selected_mode: ModeSelection::new(mode),
        }
    }

    /// Gets a new end screen
    pub fn new_end_screen(wpm: f64, accuracy: usize) -> Self {
        Screen::EndScreen { wpm, accuracy }
    }
}

struct Toast {
    messages: Vec<ToastMessage>,
}

struct Config {}

pub fn update(model: &mut AppModel, msg: Message) -> Vec<Action> {
    let mut actions = vec![];

    match msg {
        Message::Tick => {}
        Message::Key(key) => {
            match &mut model.screen {
                Screen::TypingTest {
                    typing_test,
                    selected_mode,
                    ..
                } => match key.code {
                    KeyCode::Char(c) => {
                        typing_test.start();

                        let has_ended = typing_test.on_type(c);
                        if has_ended {
                            let wpm = typing_test.net_wpm();
                            let accuracy = typing_test.accuracy();

                            if let Some(elapsed) = typing_test.elapsed_since_start_sec() {
                                actions.push(Action::Message(Message::HistoryPush((
                                    elapsed.as_secs_f64(),
                                    wpm,
                                ))));
                            }

                            actions.push(Action::Message(Message::NewEndScreen { wpm, accuracy }));
                        }
                    }
                    KeyCode::Backspace => {
                        typing_test.on_backspace();
                    }
                    KeyCode::Tab => {
                        actions.extend([
                            Action::Message(Message::ClearHistory),
                            Action::Message(Message::NewData),
                            Action::Message(Message::NewTypingTest),
                        ]);
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        if let Some(action) =
                            handle_arrow_keys(selected_mode, &mut model.mode, key.code)
                        {
                            actions.push(action);
                        }
                    }
                    _ => {}
                },
                Screen::EndScreen { .. } => {
                    match key.code {
                        KeyCode::Char('q') => {
                            actions.push(Action::Quit);
                        }
                        KeyCode::Tab => {
                            actions.extend([
                                Action::Message(Message::ClearHistory),
                                Action::Message(Message::NewData),
                                Action::Message(Message::NewTypingTest),
                            ]);
                        }
                        _ => (),
                    };
                }
            };
        }
        Message::ToastPush(msg) => {
            model.toast.messages.push(msg);
        }
        Message::ToastPop => {
            model.toast.messages.pop();
        }
        Message::SwitchScreen(screen) => {
            model.screen = screen;
        }
        Message::ClearHistory => model.history.clear(),
        Message::NewData => model.data = model.mode.get_data(),
        Message::NewTypingTest => {
            model.screen = Screen::new_typing_test(&model.data.text, model.mode.clone())
        }
        Message::NewEndScreen { wpm, accuracy } => {
            model.screen = Screen::new_end_screen(wpm, accuracy);
        }
        Message::HistoryPush(point) => {
            model.history.push(point);
        }
    }

    actions
}

fn handle_arrow_keys(
    selected_mode: &mut ModeSelection,
    current_mode: &mut Mode,
    key: KeyCode,
) -> Option<Action> {
    match key {
        KeyCode::Left => {
            selected_mode.handle_left();
        }
        KeyCode::Right => {
            selected_mode.handle_right();
        }
        KeyCode::Up => {
            selected_mode.handle_up();
        }
        KeyCode::Down => {
            selected_mode.handle_down();
        }
        _ => {}
    }
    let selected_mode = selected_mode.selected_mode();

    if let Some(selected_mode) = selected_mode
        && selected_mode != *current_mode
    {
        *current_mode = selected_mode.clone();
        return Some(Action::UpdateConfigMode(selected_mode.clone()));
    }

    None
}
