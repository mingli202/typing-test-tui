use crossterm::event::KeyEvent;

use crate::model::Screen;
use crate::toast::ToastMessage;

pub enum Message {
    Tick,
    Key(KeyEvent),
    ToastPush(ToastMessage),
    ToastPop,
    SwitchScreen(Screen),
    ClearHistory,
    HistoryPush((f64, f64)),
    NewData,
    NewTypingTest,
    NewEndScreen { wpm: f64, accuracy: usize },
}
