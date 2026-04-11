use crossterm::event::KeyEvent;

use crate::util::config::ConfigUpdate;
use crate::util::toast::ToastAction;
use crate::{endscreen, typing_test};

pub enum Msg {
    Tick,
    Key(KeyEvent),
    ToastAction(ToastAction),
    ConfigUpdate(ConfigUpdate),
}

impl typing_test::Msg {
    pub fn from(msg: Msg) -> Option<typing_test::Msg> {
        match msg {
            Msg::Tick => Some(typing_test::Msg::Tick),
            Msg::Key(key_event) => Some(typing_test::Msg::Key(key_event.code)),
            _ => None,
        }
    }
}

impl endscreen::Msg {
    pub fn from(msg: Msg) -> Option<endscreen::Msg> {
        match msg {
            Msg::Key(key_event) => Some(endscreen::Msg::Key(key_event.code)),
            _ => None,
        }
    }
}
