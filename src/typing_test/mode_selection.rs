use ratatui::macros::{line, span, text};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::state::Mode;

#[derive(PartialEq, Clone)]
pub enum WordsOption {
    Ten,
    Twentyfive,
    Fifty,
    Hundred,
}

pub enum WordSelectionOption {
    /// When selected on "Word" and not any of the word options
    /// The placeholder keep the last selected value so when pressing down after pressing up, the
    /// previous selected will be chosen. Better UX.
    Placeholder(WordsOption),
    Selected(WordsOption),
}

impl WordsOption {
    pub fn to_num(&self) -> usize {
        match self {
            Self::Ten => 10,
            Self::Twentyfive => 25,
            Self::Fifty => 50,
            Self::Hundred => 100,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Ten => Self::Twentyfive,
            Self::Twentyfive => Self::Fifty,
            Self::Fifty => Self::Hundred,
            Self::Hundred => Self::Ten,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Ten => Self::Hundred,
            Self::Twentyfive => Self::Ten,
            Self::Fifty => Self::Twentyfive,
            Self::Hundred => Self::Fifty,
        }
    }
}

pub enum ModeOption {
    Quote,
    Words(WordSelectionOption),
}

impl ModeOption {
    pub fn from_mode(mode: Mode) -> Self {
        match mode {
            Mode::Quote => ModeOption::Quote,
            Mode::Words(10) => ModeOption::Words(WordSelectionOption::Selected(WordsOption::Ten)),
            Mode::Words(25) => {
                ModeOption::Words(WordSelectionOption::Selected(WordsOption::Twentyfive))
            }
            Mode::Words(50) => ModeOption::Words(WordSelectionOption::Selected(WordsOption::Fifty)),
            Mode::Words(100) => {
                ModeOption::Words(WordSelectionOption::Selected(WordsOption::Hundred))
            }
            _ => ModeOption::Quote,
        }
    }

    pub fn to_mode(&self) -> Option<Mode> {
        match self {
            Self::Quote => Some(Mode::Quote),
            Self::Words(w) => match w {
                WordSelectionOption::Placeholder(_) => None,
                WordSelectionOption::Selected(w) => Some(Mode::Words(w.to_num())),
            },
        }
    }
}

pub struct ModeSelection {
    selected_mode: ModeOption,
}

impl ModeSelection {
    pub fn new(initial_mode: Mode) -> Self {
        ModeSelection {
            selected_mode: ModeOption::from_mode(initial_mode),
        }
    }

    pub fn to_mode(&self) -> Option<Mode> {
        self.selected_mode.to_mode()
    }

    pub fn handle_left(&mut self) {
        self.selected_mode = match &self.selected_mode {
            ModeOption::Quote => {
                ModeOption::Words(WordSelectionOption::Placeholder(WordsOption::Ten))
            }
            ModeOption::Words(WordSelectionOption::Placeholder(_)) => ModeOption::Quote,
            ModeOption::Words(WordSelectionOption::Selected(w)) => {
                ModeOption::Words(WordSelectionOption::Selected(w.clone().prev()))
            }
        }
    }

    pub fn handle_right(&mut self) {
        self.selected_mode = match &self.selected_mode {
            ModeOption::Quote => {
                ModeOption::Words(WordSelectionOption::Placeholder(WordsOption::Ten))
            }
            ModeOption::Words(WordSelectionOption::Placeholder(_)) => ModeOption::Quote,
            ModeOption::Words(WordSelectionOption::Selected(w)) => {
                ModeOption::Words(WordSelectionOption::Selected(w.clone().next()))
            }
        }
    }

    pub fn handle_up(&mut self) {
        if let ModeOption::Words(WordSelectionOption::Selected(w)) = &self.selected_mode {
            self.selected_mode = ModeOption::Words(WordSelectionOption::Placeholder(w.clone()));
        }
    }

    pub fn handle_down(&mut self) {
        if let ModeOption::Words(WordSelectionOption::Placeholder(w)) = &self.selected_mode {
            self.selected_mode = ModeOption::Words(WordSelectionOption::Selected(w.clone()));
        }
    }
}

impl Widget for &ModeSelection {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut quote_text = span!("Quote");
        let mut word_text = span!("Words");

        let selection = match &self.selected_mode {
            ModeOption::Quote => {
                quote_text = highlight(quote_text);
                text![line![quote_text, span!(" "), word_text]]
            }
            ModeOption::Words(selected_word) => {
                let mut choices = [
                    WordsOption::Ten,
                    WordsOption::Twentyfive,
                    WordsOption::Fifty,
                    WordsOption::Hundred,
                ]
                .iter()
                .map(|w| span!(w.to_num()))
                .collect::<Vec<Span>>();

                if let WordSelectionOption::Selected(word) = selected_word
                    && let Some(chosen) = choices
                        .iter_mut()
                        .find(|choice| *choice.content == word.to_num().to_string())
                {
                    *chosen = highlight(chosen.clone());
                    word_text = word_text.fg(Color::Black).bg(Color::DarkGray);
                } else {
                    word_text = highlight(word_text);
                }

                let choices: Vec<Span> =
                    itertools::Itertools::intersperse(choices.into_iter(), span!(" ")).collect();

                text![
                    line![quote_text, span!(" "), word_text],
                    span!(" "),
                    Line::from(choices)
                ]
            }
        };

        selection.centered().render(area, buf);
    }
}

fn highlight(text: Span) -> Span {
    text.fg(Color::Black).bg(Color::White)
}
