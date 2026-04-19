use crate::endscreen::EndScreenModel;
use crate::model::{Mode, Screen, SharedModel};
use crate::typing::TypingModel;
use crate::util::data_provider::DataProvider;

pub enum Action {
    Quit,
    ModeChange(Mode),
    SwitchScreen(Screen),
}

impl Action {
    pub fn new_typing_screen(
        shared_model: &mut SharedModel,
        data_provider: &DataProvider,
    ) -> Action {
        shared_model.history.clear();
        shared_model.data = data_provider.get_data_from_mode(&shared_model.mode);
        let text = &shared_model.data.text;
        Action::SwitchScreen(Screen::Typing(TypingModel::new(
            text,
            shared_model.mode.clone(),
        )))
    }

    pub fn new_end_screen(final_wpm: f64, accuracy: usize) -> Action {
        Action::SwitchScreen(Screen::End(EndScreenModel::new(final_wpm, accuracy)))
    }
}
