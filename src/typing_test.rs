use std::fmt::Display;
use std::time::Instant;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
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

    /// Whether this letter is right!
    pub fn is_error(&self) -> bool {
        match self.typed_letter {
            TypedState::Typed(c) => c != self.letter,
            _ => true,
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
            .filter(|letter| matches!(letter.typed_letter, TypedState::Typed(_)))
            .count()
    }
}

impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.letters
                .iter()
                .map(|letter| letter.letter)
                .collect::<String>()
        )
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
        let word_len = curr_word.letters_len();

        if self.letter_index >= word_len {
            curr_word.push(Letter {
                letter: c,
                typed_letter: TypedState::Extra,
                char_id: word_len,
                word_id: self.word_index,
            });
        } else {
            let curr_letter = &mut curr_word.letters[self.letter_index];
            curr_letter.typed_letter = TypedState::Typed(c);

            let is_last_word_error = curr_word.is_error();
            let is_at_last_letter_of_last_word =
                self.word_index >= self.words.len() - 1 && self.letter_index >= word_len - 1;

            if is_at_last_letter_of_last_word && !is_last_word_error {
                return true;
            }
        }

        self.letter_index += 1;

        false
    }

    /// Gets the numbers of wrong words
    pub fn n_wrongs(&self) -> usize {
        self.words.iter().filter(|word| word.is_error()).count()
    }

    /// Total number of letters typed excluding extras
    pub fn n_letters_typed(&self) -> usize {
        self.words.iter().map(|word| word.n_letters_typed()).sum()
    }

    /// Starts the typing test timer
    pub fn start(&mut self) {
        self.started = true;
        self.time_started = Instant::now();
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

    /// Handles when the user backspace.
    /// Decrement letter_index by 1.
    /// If it's the start a word, go back to the previous word
    /// and pick up where the last letter was typed.
    /// If it's the start, do nothing.
    /// Resets typed letter state to NotTyped.
    pub fn handle_backspace(&mut self) {
        let is_first_letter = self.letter_index == 0;
        if is_first_letter {
            let is_first_word = self.word_index == 0;
            if is_first_word {
                return;
            }

            self.word_index -= 1;
            self.letter_index = self.words[self.word_index].last_typed_letter_index;
        } else {
            self.letter_index -= 1;
        }

        if let Some(letter) = self.get_curr_letter_mut() {
            if matches!(letter.typed_letter, TypedState::Extra) {
                if let Some(word) = self.get_curr_word_mut() {
                    word.letters.pop();
                }
            } else {
                letter.typed_letter = TypedState::NotTyped;
            }
        }
    }

    /// Get current word.
    pub fn get_curr_word(&self) -> Option<&Word> {
        self.words.get(self.word_index)
    }

    /// Get current word.
    pub fn get_curr_word_mut(&mut self) -> Option<&mut Word> {
        self.words.get_mut(self.word_index)
    }

    /// Get current letter
    pub fn get_curr_letter(&self) -> Option<&Letter> {
        self.get_curr_word()
            .and_then(|word| word.letters.get(self.letter_index))
    }

    /// Get current letter
    pub fn get_curr_letter_mut(&mut self) -> Option<&mut Letter> {
        let letter_index = self.letter_index;
        self.get_curr_word_mut()
            .and_then(|word| word.letters.get_mut(letter_index))
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

    #[test]
    fn on_type_single_char() {
        let mut test = TypingTest::new("Hello world!");
        let did_end = test.on_type('H');

        assert_eq!(did_end, false, "should not have ended");
        assert_eq!(test.words[0].is_error(), true, "word is not complete")
    }

    #[test]
    fn on_type_one_word() {
        let mut test = TypingTest::new("Hello world!");

        let did_end = "Hello".chars().any(|c| test.on_type(c));

        assert_eq!(did_end, false, "should not have ended");
        assert_eq!(test.words[0].is_error(), false, "word is complete")
    }

    #[test]
    fn on_type_with_space_in_middle() {
        let mut test = TypingTest::new("Hello world!");

        "Hel".chars().any(|c| test.on_type(c));
        let did_end = test.on_type(' ');

        assert_eq!(did_end, false, "should not have ended");
        assert_eq!(test.words[0].is_error(), true, "word has skipped letter")
    }

    #[test]
    fn on_type_with_word_overshoot() {
        let mut test = TypingTest::new("Hello world!");

        "Hellow".chars().any(|c| test.on_type(c));
        let did_end = test.on_type('o');

        assert_eq!(did_end, false, "should not have ended");
        assert_eq!(test.words[0].is_error(), true, "word has extra letter");
        assert_eq!(test.words[0].to_string(), "Hellowo");
    }

    #[test]
    fn on_type_all() {
        let mut test = TypingTest::new("Hello world!");

        "Hello world".chars().any(|c| test.on_type(c));
        let did_end = test.on_type('!');

        assert_eq!(
            did_end, true,
            "should have ended on the last char of the last word"
        );
        assert_eq!(test.words[0].is_error(), false, "word is valid");
        assert_eq!(test.words[1].is_error(), false, "word is valid");
    }

    #[test]
    fn on_type_all_and_last_word_error() {
        let mut test = TypingTest::new("Hello world!");

        "Hello worlk".chars().any(|c| test.on_type(c));
        let did_end = test.on_type('!');

        assert_eq!(did_end, false, "should not end on last word if has error");
        assert_eq!(test.words[0].is_error(), false, "word is valid");
        assert_eq!(test.words[1].is_error(), true, "contains error");
    }

    #[test]
    fn on_type_all_and_last_word_overflow() {
        let mut test = TypingTest::new("Hello world!");

        "Hello worlkkkk".chars().any(|c| test.on_type(c));
        let did_end = test.on_type('!');

        assert_eq!(did_end, false, "should not end on last word if has error");
        assert_eq!(test.words[0].is_error(), false, "word is valid");
        assert_eq!(test.words[1].is_error(), true, "contains error");
    }

    #[test]
    fn on_type_all_and_last_word_error_but_space() {
        let mut test = TypingTest::new("Hello world!");

        "Hello worlk!".chars().any(|c| test.on_type(c));
        let did_end = test.on_type(' ');

        assert_eq!(did_end, true, "should not end on last word if has error");
        assert_eq!(test.words[0].is_error(), false, "word is valid");
        assert_eq!(test.words[1].is_error(), true, "contains error");
    }

    #[test]
    fn handle_backspace_at_start() {
        let mut test = TypingTest::new("Hello world!");

        test.handle_backspace();

        assert_eq!(test.get_curr_word().unwrap().to_string(), "Hello");
    }

    #[test]
    fn handle_backspace_at_middle_of_word() {
        let mut test = TypingTest::new("abcde fghi");

        "wers".chars().any(|c| test.on_type(c));
        test.handle_backspace();

        assert_eq!(test.get_curr_letter().unwrap().letter, 'd');
        assert_eq!(
            test.get_curr_word()
                .unwrap()
                .letters
                .iter()
                .map(|letter| letter.typed_letter.clone())
                .collect::<Vec<TypedState>>(),
            vec![
                TypedState::Typed('w'),
                TypedState::Typed('e'),
                TypedState::Typed('r'),
                TypedState::NotTyped,
                TypedState::NotTyped,
            ]
        )
    }

    #[test]
    fn handle_backspace_after_overshoot() {
        let mut test = TypingTest::new("abcde fghi");

        "abcdefgi".chars().any(|c| test.on_type(c));
        test.handle_backspace();

        assert_eq!(test.letter_index, 7);
        assert_eq!(
            test.get_curr_word()
                .unwrap()
                .letters
                .iter()
                .map(|letter| letter.typed_letter.clone())
                .collect::<Vec<TypedState>>(),
            vec![
                TypedState::Typed('a'),
                TypedState::Typed('b'),
                TypedState::Typed('c'),
                TypedState::Typed('d'),
                TypedState::Typed('e'),
                TypedState::Extra,
                TypedState::Extra,
            ]
        )
    }

    #[test]
    fn handle_backspace_after_space_at_middle_of_word() {
        let mut test = TypingTest::new("abcde fghi");

        "wer".chars().any(|c| test.on_type(c));
        test.handle_space();
        test.handle_backspace();

        assert_eq!(test.get_curr_letter().unwrap().letter, 'd');
    }

    #[test]
    fn handle_backspace_after_complete_word() {
        let mut test = TypingTest::new("abcde fghi");

        "abcde ".chars().any(|c| test.on_type(c));
        test.handle_backspace();

        assert_eq!(test.word_index, 0);
        assert_eq!(test.letter_index, 5);
    }

    #[test]
    fn n_wrongs() {
        let mut test = TypingTest::new("Hello world!");

        "Hel world!".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.n_wrongs(), 1);
    }

    #[test]
    fn n_letters_typed() {
        let mut test = TypingTest::new("Hello world!");

        "Hel waold!asdf".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.n_letters_typed(), 9);
    }
}
