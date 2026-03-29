use std::time::Instant;

#[derive(Debug)]
pub enum TypedState {
    Typed(char),
    NotTyped,
    Extra,
}

/// Represents a single letter of a word
#[derive(Debug)]
pub struct Letter {
    letter: char,

    /// states for the letter.
    /// used to style this letter white (typed), red (error), gray (not typed)
    typed_letter: TypedState,
    char_id: usize,
    word_id: usize,
}

impl Letter {
    pub fn new(letter: char, char_id: usize, word_id: usize) -> Self {
        Letter {
            letter,
            typed_letter: TypedState::NotTyped,
            char_id,
            word_id,
        }
    }
}

/// Represent a single word of the text to type
#[derive(Debug)]
pub struct Word {
    id: usize,
    letters: Vec<Letter>,

    /// The underlying word. Kept so we can easily render the word
    word: String,

    /// Which letter the user last typed
    last_typed_letter_index: usize,
}

impl Word {
    pub fn len(&self) -> usize {
        self.letters.len()
    }
}

impl Word {
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
        self.letters
            .iter()
            .map(|letter| letter.letter)
            .collect::<String>()
            == self.word
    }

    /// Push a letter to the word
    pub fn push(&mut self, letter: Letter) {
        self.letters.push(letter)
    }
}

/// Represents a single typing test
pub struct TypingTest {
    /// All the words of the text to type
    ///
    ///  0        1        2       3     4      5       6        7
    /// [[hello], [world], [this], [is], [the], [best], [thing], [ever]]
    ///   01234    01234    0123    01    012    0123    01234    0123
    words: Vec<Word>,

    /// The current word the user is typing
    word_index: usize,

    /// The current letter in the current word to be typed
    letter_index: usize,

    /// When the test has started
    time_started: Instant,

    /// Whether the test has started
    started: bool,

    /// How many wrong words
    wrongs: usize,

    /// How many characters typed in total (includes spaces)
    n_letter_typed: i32,
}

impl TypingTest {
    pub fn new(text: &str) -> Self {
        let words: Vec<Word> = text
            .split(" ")
            .enumerate()
            .map(|(id, word)| Word::new(word, id))
            .collect();

        TypingTest {
            word_index: 0,
            letter_index: 0,
            time_started: Instant::now(),
            started: false,
            words,
            wrongs: 0,
            n_letter_typed: 0,
        }
    }

    /// Processes the typed character. Returns whether the test is done.
    /// - Moves the cursor to the next character.
    /// - If letter is wrong, the current word is marked as errored.
    /// - If at the end of current word, expects a space character. Otherwise, overshoot the
    ///   current word and mark it as wrong.
    /// - Space completes the current word and goes to next word. If it's at the last word,
    ///   it will terminate the test. If the current word is incomplete, it will be marked as errored.
    pub fn on_type(&mut self, c: char) -> bool {
        if c == ' ' {
            return self.handle_space();
        }

        let curr_word = &mut self.words[self.word_index];
        let len = curr_word.len();

        if self.letter_index >= len {
            curr_word.push(Letter {
                letter: c,
                typed_letter: TypedState::Extra,
                char_id: len,
                word_id: self.word_index,
            });
        } else {
            let curr_letter = &mut curr_word.letters[self.letter_index];
            curr_letter.typed_letter = TypedState::Typed(c);
        }

        self.letter_index += 1;

        false
    }

    /// Handle the space character
    /// Moves the cursor to the next word and reset the letter index to 0
    /// If it's the last word, mark it as error and end the test
    fn handle_space(&mut self) -> bool {
        let len = self.words.len();

        let curr_word = &mut self.words[self.word_index];
        curr_word.last_typed_letter_index = self.letter_index;

        let is_last_word = self.word_index >= len - 1;

        if is_last_word {
            return true;
        }

        self.word_index += 1;
        self.letter_index = 0;

        false
    }
}

#[cfg(test)]
mod typing_test_test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn typing_test_constructor() {
        let test = TypingTest::new("Hello world!");

        assert_eq!(test.words.len(), 2)
    }

    #[test]
    fn handle_space_middle_of_word() {
        let mut test = TypingTest::new("Hello world!");
        test.letter_index = 2;

        let did_end = test.handle_space();

        assert_eq!(test.word_index, 1, "should have gone to next word");
        assert_eq!(test.letter_index, 0, "letter index should be reset");
        assert_eq!(
            test.words[0].is_error(),
            true,
            "since it's not the end of the word, a <space> is a wrong character"
        );
        assert_eq!(
            test.words[0].last_typed_letter_index, 2,
            "where you left off"
        );
        assert_eq!(did_end, false, "should not have ended");
    }

    #[test]
    fn handle_space_end_of_word() {
        let mut test = TypingTest::new("Hello world!");
        test.word_index = 0;
        test.letter_index = 5;
        test.words[0]
            .letters
            .iter_mut()
            .for_each(|letter| letter.typed_letter = TypedState::Typed(letter.letter));

        test.handle_space();

        let word = &test.words[0];

        assert_eq!(word.is_error(), false, "end of word should expect a space");
    }

    #[test]
    fn handle_space_last_word() {
        let mut test = TypingTest::new("Hello world!");
        test.word_index = 1;
        test.letter_index = 4;

        let did_end = test.handle_space();

        let word = &test.words[1];

        assert_eq!(did_end, true, "should have ended the test");
        assert_eq!(word.is_error(), true, "should have errored the last word")
    }
}
