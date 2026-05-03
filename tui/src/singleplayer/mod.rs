use serde::{Deserialize, Serialize};

use crate::util::data_provider::Data;

use self::endscreen::EndScreenModel;
use self::typing::TypingModel;

pub mod endscreen;
pub mod typing;

#[derive(Clone, PartialEq, Serialize, Deserialize, Default, Debug)]
pub enum Mode {
    #[default]
    Quote,

    /// can only be either 10, 25, 50, or 100
    Words(usize),

    /// can only be either 15, 30, 60, or 120 seconds
    Time(usize),
}

pub struct SharedModel {
    pub mode: Mode,
    // (time, wpm)
    pub history: Vec<(f64, f64)>,
    pub data: Data,
}

pub enum SinglePlayerScreen {
    Typing(TypingModel),
    End(EndScreenModel),
}

pub struct SinglePlayerModel {
    shared_model: SharedModel,
    screen: SinglePlayerScreen,
}

impl SinglePlayerModel {
    pub fn new(data: Data, initial_mode: Mode) -> Self {
        let text = &data.text;

        SinglePlayerModel {
            screen: SinglePlayerScreen::Typing(TypingModel::new(text, initial_mode)),
            shared_model: SharedModel {
                mode: initial_mode,
                history: vec![],
                data,
            },
        }
    }
}
