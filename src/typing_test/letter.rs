use std::fmt::Display;

use ratatui::style::{Color, Stylize};
use ratatui::text::Span;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum TypedState {
    Typed(char),
    NotTyped,
    Extra,
}

/// Represents a single letter of a word
#[derive(Debug)]
pub struct Letter {
    /// Its letter
    pub(super) letter: char,

    /// states for the letter.
    /// used to style this letter white (typed), red (error), gray (not typed)
    pub(super) typed_state: TypedState,
}

impl Letter {
    /// Creates a new Letter with the given letter, char_id, and word_id
    pub fn new(letter: char) -> Self {
        Letter {
            letter,
            typed_state: TypedState::NotTyped,
        }
    }

    /// factory with typed letter
    pub fn with_typed_letter(self, typed_letter: TypedState) -> Self {
        Letter {
            typed_state: typed_letter,
            ..self
        }
    }

    /// Whether this letter is right!
    pub fn is_error(&self) -> bool {
        match self.typed_state {
            TypedState::Typed(c) => c != self.letter,
            _ => true,
        }
    }

    /// Gets the span representation of this letter
    pub fn to_span(&self) -> Span<'_> {
        match self.typed_state {
            TypedState::Typed(c) => Span::raw(c.to_string()).fg(if c == self.letter {
                Color::White
            } else {
                Color::Red
            }),
            TypedState::NotTyped => Span::raw(self.letter.to_string()).fg(Color::DarkGray),
            TypedState::Extra => Span::raw(self.letter.to_string()).fg(Color::Red),
        }
    }
}

impl Display for Letter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter)
    }
}
