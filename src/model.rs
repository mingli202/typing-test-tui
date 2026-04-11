use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Constraint;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::CustomEvent;
use crate::action::Action;
use crate::data::Data;
use crate::endscreen::{self, EndScreenModel};
pub use crate::msg::Msg;
use crate::typing_test::{self, TypingModel};
use crate::util::config::Config;
use crate::util::toast::{self, Toast};

#[derive(Clone, PartialEq, Serialize, Deserialize, Default, Debug)]
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

pub enum Screen {
    Typing(TypingModel),
    End(EndScreenModel),
}

pub struct SharedModel {
    pub mode: Mode,
    // (time, wpm)
    pub history: Vec<(f64, f64)>,
    pub data: Data,
    pub event_tx: UnboundedSender<CustomEvent>,
}

pub struct AppModel {
    pub exit: bool,
    toast: Toast,
    config: Config,
    screen: Screen,
    shared_model: SharedModel,
}

impl AppModel {
    pub async fn init(event_tx: UnboundedSender<CustomEvent>) -> Self {
        let config = Config::new(event_tx.clone()).await;
        let toast = Toast::new(event_tx.clone());

        let initial_mode = config.data.mode.clone();
        let data = initial_mode.get_data();
        let text = &data.text;

        AppModel {
            exit: false,
            screen: Screen::Typing(TypingModel::new(text, initial_mode.clone())),
            shared_model: SharedModel {
                mode: initial_mode,
                history: vec![],
                data,
                event_tx,
            },
            toast,
            config,
        }
    }
}

pub async fn update(model: &mut AppModel, msg: Msg) -> Option<Action> {
    match msg {
        Msg::ToastAction(action) => model.toast.handle_action(action),
        Msg::ConfigUpdate(config_update) => {
            model.config.handle_config_update(config_update).await;
        }
        _ => {
            if let Msg::Key(
                KeyEvent {
                    code: KeyCode::Esc, ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                },
            ) = msg
            {
                model.exit = true
            }

            return match &mut model.screen {
                Screen::Typing(typing_model) => typing_test::Msg::from(msg).and_then(|msg| {
                    typing_test::update(typing_model, &mut model.shared_model, msg)
                }),
                Screen::End(_) => endscreen::Msg::from(msg)
                    .and_then(|msg| endscreen::update(&mut model.shared_model, msg)),
            };
        }
    };

    None
}

pub fn view(model: &AppModel, frame: &mut Frame) {
    let area = frame.area();
    let buf = frame.buffer_mut();

    let centered = area.centered_horizontally(Constraint::Max(80));

    match &model.screen {
        Screen::Typing(typing_model) => {
            typing_test::view(typing_model, &model.shared_model, centered, buf)
        }
        Screen::End(endscreen_model) => {
            endscreen::view(endscreen_model, &model.shared_model, centered, buf)
        }
    };

    toast::view(&model.toast, area, buf);
}

pub fn handle_action(model: &mut AppModel, action: Action) {
    match action {
        Action::Quit => model.exit = true,
        Action::SwitchScreen(screen) => model.screen = screen,
        _ => {}
    };
}
