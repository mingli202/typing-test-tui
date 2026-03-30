use crate::typing_test::letter::TypedState;

use super::letter::Letter;

/// Represent a single word of the text to type
#[derive(Debug)]
pub struct Word {
    /// Index of the word in the typing test
    id: usize,

    /// Its letters
    letters: Vec<Letter>,

    /// The underlying word. Kept so we can easily render the word
    word: String,

    /// Which letter the user last typed
    last_typed_letter_index: usize,
}

impl Word {
    /// Creates a new Word from the given string and id
    pub fn new(text: &str, id: usize) -> Word {
        Word {
            letters: text
                .chars()
                .enumerate()
                .map(|(i, letter)| Letter::new(letter, i, id))
                .collect(),
            id,
            word: text.to_string(),
            last_typed_letter_index: 0,
        }
    }

    /// Whether any letter is errored
    /// If a word is errored, there will be a red underline
    /// This error is only computed for typed words (e.g. every word before the current word)
    pub fn is_error(&self) -> bool {
        self.letters.iter().any(|letter| letter.is_error())
    }

    /// Push a letter to the word
    pub fn push(&mut self, letter: Letter) {
        self.letters.push(letter)
    }

    /// Pops the last letter
    pub fn pop(&mut self) -> Option<Letter> {
        self.letters.pop()
    }

    /// Gets the length of all its typed and untyped letters
    pub fn letters_len(&self) -> usize {
        self.letters.len()
    }

    /// Gets the actual length of the word to type
    pub fn actual_len(&self) -> usize {
        self.word.len()
    }

    /// Gets the number of letter typed excluding extras
    pub fn n_letters_typed(&self) -> usize {
        self.letters
            .iter()
            .filter(|letter| matches!(letter.typed_state(), TypedState::Typed(_)))
            .count()
    }

    /// String representation but with only typed letters
    pub fn to_string_typed(&self) -> String {
        self.letters
            .iter()
            .filter_map(|letter| match letter.typed_state() {
                TypedState::Typed(c) => Some(c),
                _ => None,
            })
            .collect::<String>()
    }
}

impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.letters
                .iter()
                .map(|letter| letter.to_string())
                .collect::<String>()
        )
    }
}
