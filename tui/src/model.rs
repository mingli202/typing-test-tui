use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
pub use crate::msg::Msg;
use crate::singleplayer::SinglePlayerModel;
use crate::util::config::{Config, ConfigUpdate};
use crate::util::data_provider::DataProvider;
use crate::util::toast::{self, Toast};
use crate::{CustomEvent, singleplayer};

pub enum Screen {
    SinglePlayer(SinglePlayerModel),
    Multiplayer,
}

pub struct AppModel {
    pub exit: bool,
    toast: Toast,
    config: Config,
    screen: Screen,
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

        Ok(AppModel {
            exit: false,
            screen: Screen::SinglePlayer(SinglePlayerModel::new(data, initial_mode)),
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
                Screen::SinglePlayer(singleplayer_model) => {
                    singleplayer::update(singleplayer_model, &model.data_provider, msg)
                }
                Screen::Multiplayer => None,
            };
        }
    };

    None
}

pub fn view(model: &AppModel, frame: &mut Frame) {
    let area = frame.area();
    let buf = frame.buffer_mut();

    match &model.screen {
        Screen::SinglePlayer(singleplayer_model) => {
            singleplayer::view(singleplayer_model, area, buf)
        }
        Screen::Multiplayer => {}
    };

    toast::view(&model.toast, area, buf);
}

pub fn handle_action(model: &mut AppModel, action: Action) -> Option<Action> {
    match action {
        Action::Quit => model.exit = true,
        Action::SwitchScreen(screen) => model.screen = screen,
        Action::ConfigModeUpdate(mode) => {
            model
                .config
                .handle_config_update(ConfigUpdate::Mode(mode.clone()));
        }
    };

    None
}
