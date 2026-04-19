use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Constraint;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::CustomEvent;
use crate::action::Action;
use crate::endscreen::{self, EndScreenModel};
pub use crate::msg::Msg;
use crate::typing::{self, TypingModel};
use crate::util::config::{Config, ConfigUpdate};
use crate::util::data_provider::{Data, DataProvider};
use crate::util::toast::{self, Toast};

#[derive(Clone, PartialEq, Serialize, Deserialize, Default, Debug)]
pub enum Mode {
    #[default]
    Quote,

    /// can only be either 10, 25, 50, or 100
    Words(usize),

    /// can only be either 15, 30, 60, or 120 seconds
    Time(usize),
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
    data_provider: DataProvider,
}

impl AppModel {
    pub async fn new(
        event_tx: UnboundedSender<CustomEvent>,
        words_path: Option<String>,
        quotes_path: Option<String>,
    ) -> color_eyre::Result<Self> {
        let config = Config::new(event_tx.clone()).await;
        let toast = Toast::new(event_tx.clone());
        let data_provider = DataProvider::new(words_path, quotes_path)?;

        let initial_mode = config.data.mode.clone();
        let data = data_provider.get_data_from_mode(&initial_mode);
        let text = &data.text;

        Ok(AppModel {
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
            data_provider,
        })
    }
}

pub fn update(model: &mut AppModel, msg: Msg) -> Option<Action> {
    match msg {
        Msg::ToastAction(action) => model.toast.handle_action(action),
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
                return Some(Action::Quit);
            }

            return match &mut model.screen {
                Screen::Typing(typing_model) => typing::Msg::from(msg).and_then(|msg| {
                    typing::update(
                        typing_model,
                        &mut model.shared_model,
                        &model.data_provider,
                        msg,
                    )
                }),
                Screen::End(_) => endscreen::Msg::from(msg).and_then(|msg| {
                    endscreen::update(&mut model.shared_model, &model.data_provider, msg)
                }),
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
            typing::view(typing_model, &model.shared_model, centered, buf)
        }
        Screen::End(endscreen_model) => {
            endscreen::view(endscreen_model, &model.shared_model, centered, buf)
        }
    };

    toast::view(&model.toast, area, buf);
}

pub fn handle_action(model: &mut AppModel, action: Action) -> Option<Action> {
    match action {
        Action::Quit => model.exit = true,
        Action::SwitchScreen(screen) => model.screen = screen,
        Action::ModeChange(mode) => {
            model
                .config
                .handle_config_update(ConfigUpdate::Mode(mode.clone()));

            if let Screen::Typing(typing_model) = &mut model.screen {
                return typing::update(
                    typing_model,
                    &mut model.shared_model,
                    &model.data_provider,
                    typing::Msg::UpdateMode(mode),
                );
            }
        }
    };

    None
}
