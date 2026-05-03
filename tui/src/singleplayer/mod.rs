use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};

use crate::msg::Msg;
use crate::util::data_provider::{Data, DataProvider};

use self::endscreen::EndScreenModel;
use self::typing::TypingModel;

mod action;
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
            screen: SinglePlayerScreen::Typing(TypingModel::new(text, initial_mode.clone())),
            shared_model: SharedModel {
                mode: initial_mode,
                history: vec![],
                data,
            },
        }
    }
}

pub fn update(
    model: &mut SinglePlayerModel,
    data_provider: &DataProvider,
    msg: Msg,
) -> Option<crate::action::Action> {
    let mut maybe_action = match &mut model.screen {
        SinglePlayerScreen::Typing(typing_model) => typing::Msg::from(msg).and_then(|msg| {
            typing::update(typing_model, &mut model.shared_model, data_provider, msg)
        }),
        SinglePlayerScreen::End(_) => endscreen::Msg::from(msg)
            .and_then(|msg| endscreen::update(&mut model.shared_model, data_provider, msg)),
    };

    while let Some(action) = maybe_action {
        if let action::Action::Root(root_action) = action {
            return Some(root_action);
        }

        maybe_action = handle_action(model, data_provider, action)
    }

    None
}

pub fn view(model: &SinglePlayerModel, area: Rect, buf: &mut Buffer) {
    let centered = area.centered_horizontally(Constraint::Max(80));

    match &model.screen {
        SinglePlayerScreen::Typing(typing_model) => {
            typing::view(typing_model, &model.shared_model, centered, buf)
        }
        SinglePlayerScreen::End(endscreen_model) => {
            endscreen::view(endscreen_model, &model.shared_model, centered, buf)
        }
    };
}

pub fn handle_action(
    model: &mut SinglePlayerModel,
    data_provider: &DataProvider,
    action: action::Action,
) -> Option<action::Action> {
    match action {
        action::Action::ModeChange(mode) => {
            model.shared_model.mode = mode.clone();

            model.shared_model.history.clear();
            model.shared_model.data = data_provider.get_data_from_mode(&model.shared_model.mode);
            let text = &model.shared_model.data.text;

            model.screen = SinglePlayerScreen::Typing(TypingModel::new(text, mode.clone()));

            return Some(action::Action::Root(
                crate::action::Action::ConfigModeUpdate(mode),
            ));
        }
        action::Action::SwitchScreen(screen) => model.screen = screen,
        action::Action::Root(_) => return Some(action),
    }

    None
}
