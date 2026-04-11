use crate::endscreen::EndScreenModel;
use crate::model::{Mode, Screen, SharedModel};
use crate::typing_test::TypingModel;

pub enum Action {
    Quit,
    UpdateConfigMode(Mode),
    SwitchScreen(Screen),
}

impl Action {
    pub fn new_typing_screen(shared_model: &mut SharedModel) -> Action {
        shared_model.history.clear();
        shared_model.data = shared_model.mode.get_data();
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
