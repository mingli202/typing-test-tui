use crossterm::event::KeyEvent;

use crate::util::toast::ToastAction;
use crate::{endscreen, typing};

pub enum Msg {
    Tick,
    Key(KeyEvent),
    ToastAction(ToastAction),
}

impl typing::Msg {
    pub fn from(msg: Msg) -> Option<typing::Msg> {
        match msg {
            Msg::Tick => Some(typing::Msg::Tick),
            Msg::Key(key_event) => Some(typing::Msg::Key(key_event.code)),
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
