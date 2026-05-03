use crate::util::data_provider::DataProvider;

use super::endscreen::EndScreenModel;
use super::typing::TypingModel;
use super::{Mode, SharedModel, SinglePlayerScreen};

pub enum Action {
    Root(crate::action::Action),
    ModeChange(Mode),
    SwitchScreen(SinglePlayerScreen),
}

impl Action {
    pub fn new_typing_screen(
        shared_model: &mut SharedModel,
        data_provider: &DataProvider,
    ) -> Action {
        shared_model.history.clear();
        shared_model.data = data_provider.get_data_from_mode(&shared_model.mode);
        let text = &shared_model.data.text;
        Action::SwitchScreen(SinglePlayerScreen::Typing(TypingModel::new(
            text,
            shared_model.mode.clone(),
        )))
    }

    pub fn new_end_screen(final_wpm: f64, accuracy: usize) -> Action {
        Action::SwitchScreen(SinglePlayerScreen::End(EndScreenModel::new(
            final_wpm, accuracy,
        )))
    }
}
