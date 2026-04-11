use std::fmt::Display;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::model::Mode;
use crate::util::selection::{self, Selection, SelectionItem};

#[derive(PartialEq, Clone)]
pub enum ModeSelectionOption {
    Quote,
    WordsPlaceholder,
    Words(usize),
    TimePlaceholder,
    Time(usize),
}

impl ModeSelectionOption {
    pub fn to_mode(&self) -> Option<Mode> {
        match self {
            Self::Words(n) => Some(Mode::Words(*n)),
            Self::Time(n) => Some(Mode::Time(*n)),
            Self::Quote => Some(Mode::Quote),
            _ => None,
        }
    }

    pub fn from_mode(mode: Mode) -> Self {
        match mode {
            Mode::Quote => Self::Quote,
            Mode::Words(n) => Self::Words(n),
            Mode::Time(n) => Self::Time(n),
        }
    }
}

impl Display for ModeSelectionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Quote => "Quote".to_string(),
                Self::WordsPlaceholder => "Words".to_string(),
                Self::TimePlaceholder => "Time".to_string(),
                Self::Words(n) => n.to_string(),
                Self::Time(t) => t.to_string(),
            }
        )
    }
}

pub struct ModeSelection {
    selection: Selection<ModeSelectionOption>,
}

impl ModeSelection {
    pub fn new(initial_mode: Mode) -> Self {
        let mut selection = Selection::new(vec![
            SelectionItem::new(ModeSelectionOption::Quote),
            SelectionItem::new(ModeSelectionOption::WordsPlaceholder).children(vec![
                SelectionItem::new(ModeSelectionOption::Words(10)),
                SelectionItem::new(ModeSelectionOption::Words(25)),
                SelectionItem::new(ModeSelectionOption::Words(50)),
                SelectionItem::new(ModeSelectionOption::Words(100)),
            ]),
            SelectionItem::new(ModeSelectionOption::TimePlaceholder).children(vec![
                SelectionItem::new(ModeSelectionOption::Time(15)),
                SelectionItem::new(ModeSelectionOption::Time(30)),
                SelectionItem::new(ModeSelectionOption::Time(60)),
                SelectionItem::new(ModeSelectionOption::Time(120)),
            ]),
        ]);

        let selected_mode = ModeSelectionOption::from_mode(initial_mode);
        selection.select(selected_mode);

        ModeSelection { selection }
    }

    pub fn selected_mode(&self) -> Option<Mode> {
        self.selection
            .get_selected_item()
            .and_then(|item| item.to_mode())
    }

    pub fn handle_left(&mut self) {
        self.selection.left();
    }

    pub fn handle_right(&mut self) {
        self.selection.right();
    }

    pub fn handle_up(&mut self) {
        self.selection.up();
    }

    pub fn handle_down(&mut self) {
        self.selection.down();
    }
}

/// Render the given mode selection
pub fn view_mode_selection(mode_selection: &ModeSelection, area: Rect, buf: &mut Buffer) {
    let paragraph = selection::get_widget(&mode_selection.selection).centered();
    paragraph.render(area, buf);
}
