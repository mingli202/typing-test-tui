use ratatui::macros::{line, span, text};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::state::Mode;

#[derive(PartialEq)]
pub enum WordsOption {
    Ten,
    Twentyfive,
    Fifty,
    Hundred,
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
}

pub enum ModeOption {
    Quote,
    Words(Option<WordsOption>),
}

impl ModeOption {
    pub fn from_mode(mode: Mode) -> Self {
        match mode {
            Mode::Quote => ModeOption::Quote,
            Mode::Words(10) => ModeOption::Words(Some(WordsOption::Ten)),
            Mode::Words(25) => ModeOption::Words(Some(WordsOption::Twentyfive)),
            Mode::Words(50) => ModeOption::Words(Some(WordsOption::Fifty)),
            Mode::Words(100) => ModeOption::Words(Some(WordsOption::Hundred)),
            _ => ModeOption::Quote,
        }
    }

    pub fn to_mode(&self) -> Option<Mode> {
        match self {
            Self::Quote => Some(Mode::Quote),
            Self::Words(w) => w.as_ref().map(|n| Mode::Words(n.to_num())),
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

                if let Some(word) = selected_word
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
