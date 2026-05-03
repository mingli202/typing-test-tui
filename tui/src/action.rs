use crate::model::Screen;
use crate::singleplayer::Mode;

pub enum Action {
    Quit,
    ConfigModeUpdate(Mode),
    SwitchScreen(Screen),
}
