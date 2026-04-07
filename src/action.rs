use crate::state::Mode;

pub enum Action {
    None,
    Quit,
    UpdateMode(Mode),
}
