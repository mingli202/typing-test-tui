use std::fmt::Display;
use std::time::{Duration, Instant};

use itertools::Itertools;
use ratatui::macros::line;
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};

use super::letter::{Letter, TypedState};
use super::word::Word;

/// Represents a single typing test
pub struct TypingTest {
    /// All the words of the text to type
    ///
    ///  0        1        2       3     4      5       6        7
    /// [[hello], [world], [this], [is], [the], [best], [thing], [ever]]
    ///   01234    01234    0123    01    012    0123    01234    0123
    words: Vec<Word>,

    /// The current word the user is typing
    pub(super) word_index: usize,

    /// The current letter in the current word to be typed
    letter_index: usize,

    /// When the test has started
    time_started: Option<Instant>,

    /// When the test as ended
    time_ended: Option<Instant>,

    /// Number of wrong words up to the current typed word (exclusively)
    n_wrongs: usize,

    /// Number of letters typed. Does not include untyped letters or extra letters
    n_letters_typed: usize,
}

impl TypingTest {
    /// Creates a new TypingTest with the given &str
    pub fn new(text: &str) -> Self {
        let words: Vec<Word> = text.split(" ").map(Word::new).collect();

        TypingTest {
            word_index: 0,
            letter_index: 0,
            time_started: None,
            time_ended: None,
            words,
            n_wrongs: 0,
            n_letters_typed: 0,
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
        if self.is_done() {
            return true;
        }

        let is_done = if c == ' ' {
            self.on_space()
        } else {
            let curr_word = &mut self.words[self.word_index];
            let word_len = curr_word.letters_len();

            let is_overshoot = self.letter_index >= word_len;
            if is_overshoot {
                curr_word.push(Letter::new(c).with_typed_letter(TypedState::Extra));
            } else {
                let curr_letter = curr_word.get_letter_mut(self.letter_index).unwrap();
                curr_letter.typed_state = TypedState::Typed(c);

                self.n_letters_typed += 1;
            }

            let is_last_word_error = curr_word.is_error();
            let is_at_last_letter_of_last_word =
                self.word_index >= self.words.len() - 1 && self.letter_index >= word_len - 1;

            self.letter_index += 1;
            is_at_last_letter_of_last_word && !is_last_word_error
        };

        if is_done {
            self.time_ended = Some(Instant::now());

            // Move to the end of the words so that n_wrongs counts it
            self.word_index = self.words.len();
        }

        is_done
    }

    /// Gets the numbers of wrong words up to the current word the user is typing
    /// Do not include the word that's being typed
    pub fn n_wrongs(&self) -> usize {
        self.n_wrongs
    }

    /// Total number of letters typed excluding extras up to the currently typed word
    /// Include the word that's being typed
    pub fn letters_typed(&self) -> usize {
        self.n_letters_typed
    }

    /// Starts the typing test timer if it hasn't been started
    pub fn start(&mut self) {
        if self.has_started() {
            return;
        }

        self.time_started = Some(Instant::now());
    }

    /// Gets the current wpm at the time called
    pub fn net_wpm(&self) -> f64 {
        let wpm = match self.elapsed_since_start_sec() {
            Some(elapsed) if elapsed > Duration::from_secs(1) => {
                let current_typed_words =
                    self.letters_typed() as f64 / 5.0 - self.n_wrongs() as f64;
                60.0 * current_typed_words / elapsed.as_secs_f64()
            }
            _ => 0.0,
        };

        if wpm < 0.0 { 0.0 } else { wpm }
    }

    /// Whether the test has started
    pub fn has_started(&self) -> bool {
        self.time_started.is_some()
    }

    /// Whether the test is done
    pub fn is_done(&self) -> bool {
        self.time_ended.is_some()
    }

    /// Handles when the user backspace.
    /// Decrement letter_index by 1.
    /// If it's the start a word, go back to the previous word
    /// and pick up where the last letter was typed.
    /// If it's the start, do nothing.
    /// Resets typed letter state to NotTyped.
    pub fn on_backspace(&mut self) {
        let is_first_letter = self.letter_index == 0;
        if is_first_letter {
            let is_first_word = self.word_index == 0;
            if is_first_word {
                return;
            }

            self.word_index -= 1;
            self.letter_index = self.words[self.word_index].last_typed_letter_index;

            if let Some(word) = self.get_curr_word()
                && word.is_error()
            {
                self.n_wrongs -= 1;
            }

            // decrement the space character that completed the previous word
            self.n_letters_typed -= 1;
        } else {
            self.letter_index -= 1;
        }

        if let Some(letter) = self.get_curr_letter_mut() {
            match letter.typed_state {
                TypedState::Typed(_) => {
                    letter.typed_state = TypedState::NotTyped;
                    self.n_letters_typed -= 1;
                }
                TypedState::Extra => {
                    self.get_curr_word_mut().and_then(|word| word.pop());
                }
                TypedState::NotTyped => (),
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
            .and_then(|word| word.get_letter(self.letter_index))
    }

    /// Get current letter
    pub fn get_curr_letter_mut(&mut self) -> Option<&mut Letter> {
        let letter_index = self.letter_index;
        self.get_curr_word_mut()
            .and_then(|word| word.get_letter_mut(letter_index))
    }

    /// Get accuracy
    pub fn accuracy(&self) -> usize {
        let total_correct_letters = self.total_correct_letters_typed();
        let total_letters = self.total_letters();

        if total_letters == 0 {
            return 0;
        }

        100 * total_correct_letters / total_letters
    }

    /// The amount of words to type
    pub fn n_words(&self) -> usize {
        self.words.len()
    }

    /// Handle the space character
    /// Moves the cursor to the next word and reset the letter index to 0
    /// If it's the last word, mark it as error and end the test
    fn on_space(&mut self) -> bool {
        let len = self.words.len();

        let curr_word = &mut self.words[self.word_index];
        curr_word.last_typed_letter_index = self.letter_index;

        if curr_word.is_error() {
            self.n_wrongs += 1;
        }

        let is_last_word = self.word_index >= len - 1;

        if is_last_word {
            return true;
        }

        self.word_index += 1;
        self.letter_index = 0;
        self.n_letters_typed += 1;

        false
    }

    /// Gets the time since start
    /// If the test has ended, use the end time as now
    pub fn elapsed_since_start_sec(&self) -> Option<Duration> {
        self.time_started.map(|start_time| {
            self.time_ended
                .map_or_else(|| start_time.elapsed(), |now| now - start_time)
        })
    }

    /// Get total correct letters
    fn total_correct_letters_typed(&self) -> usize {
        self.words
            .iter()
            .take(self.word_index)
            .map(|word| {
                word.letters
                    .iter()
                    .filter(|letter| !letter.is_error())
                    .count()
            })
            .sum::<usize>()
            + self.word_index
            + self.words.get(self.word_index).map_or(0, |word| {
                word.letters
                    .iter()
                    .filter(|letter| !letter.is_error())
                    .count()
            })
    }

    /// The total amount of characters of this test.
    fn total_letters(&self) -> usize {
        self.words
            .iter()
            .take(self.word_index)
            .map(|word| word.actual_len())
            .sum::<usize>()
            + self.word_index
            + self
                .words
                .get(self.word_index)
                .map_or(0, |word| usize::min(word.actual_len(), self.letter_index))
    }
}

impl Display for TypingTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.words.iter().map(|word| word.to_string()).join(" ")
        )
    }
}

/// Returns text representation and cursorline index
fn split_into_lines(typing_test: &TypingTest, max_width: usize) -> (Text<'_>, usize) {
    let mut lines: Vec<Line> = vec![];
    let mut current_line: Line = line![];
    let mut cursor_index = 0;

    for (i, word) in typing_test.words.iter().enumerate() {
        let mut letters = word
            .letters
            .iter()
            .map(|letter| letter.to_span())
            .collect_vec();

        // add space
        letters.push(Span::raw(" "));

        // draw cursor
        if typing_test.word_index == i {
            if let Some(letter) = letters.get_mut(typing_test.letter_index) {
                *letter = letter.clone().fg(Color::Black).bg(Color::White);
            }

            cursor_index = lines.len();
        }

        let line = Line::from(letters);
        let width_so_far = current_line.width();

        let is_overflow = width_so_far + line.width() >= max_width;
        if is_overflow {
            lines.push(current_line.clone());
            current_line = line;

            if typing_test.word_index == i {
                cursor_index += 1;
            }
        } else {
            current_line
                .spans
                .append(&mut line.into_iter().collect_vec());
        }
    }

    lines.push(current_line);

    (Text::from(lines), cursor_index)
}

pub fn view_typing_test(
    typing_test: &TypingTest,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
) {
    let (text, cursor_index) = split_into_lines(typing_test, area.width as usize);

    let offset = if cursor_index == 0 {
        0
    } else {
        cursor_index as u16 - 1
    };

    Paragraph::new(text).scroll((offset, 0)).render(area, buf);
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
    fn on_space_middle_of_word() {
        let mut test = TypingTest::new("Hello world!");
        test.letter_index = 2;

        let did_end = test.on_space();

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
    fn on_space_end_of_word() {
        let mut test = TypingTest::new("Hello world!");

        "Hello".chars().for_each(|c| {
            test.on_type(c);
        });

        test.on_space();

        let word = &test.words[0];

        assert_eq!(word.is_error(), false, "end of word should expect a space");
    }

    #[test]
    fn on_space_last_word() {
        let mut test = TypingTest::new("Hello world!");
        test.word_index = 1;
        test.letter_index = 4;

        let did_end = test.on_space();

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
    fn on_backspace_at_start() {
        let mut test = TypingTest::new("Hello world!");

        test.on_backspace();

        assert_eq!(test.get_curr_word().unwrap().to_string(), "Hello");
    }

    #[test]
    fn on_backspace_at_middle_of_word() {
        let mut test = TypingTest::new("abcde fghi");

        "wers".chars().any(|c| test.on_type(c));
        test.on_backspace();

        assert_eq!(test.letter_index, 3);
        assert_eq!(
            test.get_curr_word()
                .unwrap()
                .letters
                .iter()
                .map(|letter| letter.typed_state.clone())
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
    fn on_backspace_after_overshoot() {
        let mut test = TypingTest::new("abcde fghi");

        "abcdefgi".chars().any(|c| test.on_type(c));
        test.on_backspace();

        assert_eq!(test.letter_index, 7);
        assert_eq!(
            test.get_curr_word()
                .unwrap()
                .letters
                .iter()
                .map(|letter| letter.typed_state.clone())
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
    fn on_backspace_after_space_at_middle_of_word() {
        let mut test = TypingTest::new("abcde fghi");

        "wer".chars().any(|c| test.on_type(c));
        test.on_space();
        test.on_backspace();

        assert_eq!(test.letter_index, 3);
    }

    #[test]
    fn on_backspace_after_complete_word() {
        let mut test = TypingTest::new("abcde fghi");

        "abcde ".chars().any(|c| test.on_type(c));
        test.on_backspace();

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
    fn n_wrongs2() {
        let mut test = TypingTest::new("Hello world!");

        "Hel worlddd ".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.n_wrongs(), 2);
    }

    #[test]
    fn n_wrongs_with_backspace() {
        let mut test = TypingTest::new("Hello world!");

        "Hel ".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.n_wrongs(), 1, "should have an error");

        test.on_backspace();

        assert_eq!(test.n_wrongs(), 0, "should have removed the error");

        "lo ".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.n_wrongs(), 0);
    }

    #[test]
    fn n_wrongs_with_backspace2() {
        let mut test = TypingTest::new("Hello world! this is peak");

        "Hel world! thasdf is ewa".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(
            test.n_wrongs(),
            2,
            "should have 2 errors. current word should not be part of errors"
        );

        for _ in 0..7 {
            test.on_backspace();
        }

        assert_eq!(test.n_wrongs(), 1, "should have removed an error");

        for _ in 0..8 {
            test.on_backspace();
        }

        assert_eq!(test.n_wrongs(), 1);
    }

    #[test]
    fn total_letters_typed() {
        let mut test = TypingTest::new("Hello world!");

        "Hel waold!asdf".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(test.letters_typed(), 10);
    }

    #[test]
    fn simulate_usage() {
        let mut test = TypingTest::new("Hello World!");
        test.on_type('h');
        test.on_type('e');
        test.on_type('l');
        test.on_type('p');
        test.on_backspace();
        test.on_backspace();
        test.on_backspace();
        test.on_backspace();
        test.on_backspace();
        test.on_backspace();
        test.on_type('H');
        test.on_type('e');
        test.on_type('l');
        test.on_type('l');
        test.on_type('o');
        test.on_type(' ');
        test.on_type('w');
        test.on_type('o');
        test.on_backspace();
        test.on_backspace();
        test.on_backspace();
        test.on_type('W');
        test.on_backspace();
        test.on_type(' ');
        test.on_type('W');
        test.on_type('o');
        test.on_type('r');
        test.on_type('l');
        test.on_type('d');
        let did_end_1 = test.on_type('1');
        test.on_backspace();
        let did_end_2 = test.on_type('!');
        let did_end_3 = test.on_type('1');

        assert_eq!(
            test.words[1]
                .letters
                .iter()
                .map(|letter| letter.typed_state.clone())
                .collect::<Vec<TypedState>>(),
            vec![
                TypedState::Typed('W'),
                TypedState::Typed('o'),
                TypedState::Typed('r'),
                TypedState::Typed('l'),
                TypedState::Typed('d'),
                TypedState::Typed('!'),
            ]
        );
        assert_eq!(test.words[1].to_string_typed(), "World!", "last typed word");
        assert_eq!(
            test.words[0].is_error(),
            false,
            "first word should have no error"
        );
        assert_eq!(
            test.words[1].is_error(),
            false,
            "last word should have no error"
        );
        assert_eq!(test.word_index, 2);
        assert_eq!(test.letter_index, 6);
        assert_eq!(
            did_end_1, false,
            "should not have ended when last word is error"
        );
        assert_eq!(
            did_end_2, true,
            "should have ended after correcting himself"
        );
        assert_eq!(test.n_wrongs(), 0, "should have corrected everything");
        assert_eq!(did_end_3, true, "should be true even after ended");
    }

    #[test]
    fn elapsed_using_start() {
        let mut test = TypingTest::new("Hello World!");

        assert_eq!(test.elapsed_since_start_sec(), None);

        test.start();

        assert_eq!(
            test.elapsed_since_start_sec().unwrap() < Duration::from_secs(1),
            true
        );
    }

    #[test]
    fn elapsed_setting_start() {
        let mut test = TypingTest::new("Hello World!");

        assert_eq!(test.elapsed_since_start_sec(), None);

        test.time_started = Some(Instant::now());
        test.time_ended = test
            .time_started
            .map(|time_started| time_started + Duration::from_secs(10));

        assert_eq!(test.elapsed_since_start_sec().unwrap().as_secs(), 10);
    }

    #[test]
    fn letters_typed() {
        let mut test = TypingTest::new("Hello World!");

        "Hel Worlasdf".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(
            test.letters_typed(),
            10,
            "should not count untyped letters or extra letters"
        )
    }

    #[test]
    fn letters_typed_and_backspace() {
        let mut test = TypingTest::new("Hello World!");

        "Hel Worlasdf".chars().for_each(|c| {
            test.on_type(c);
        });

        for _ in 0..3 {
            test.on_backspace();
        }

        assert_eq!(
            test.letters_typed(),
            9,
            "should not count untyped letters or extra letters"
        );

        for _ in 0..5 {
            test.on_backspace();
        }

        assert_eq!(
            test.letters_typed(),
            4,
            "should not count untyped letters or extra letters"
        );

        test.on_backspace();
        test.on_backspace();

        assert_eq!(
            test.letters_typed(),
            2,
            "should not count untyped letters or extra letters"
        );
    }

    #[test]
    fn letters_typed_and_backspace_2() {
        let mut test = TypingTest::new("Hello World!");

        "Hello ".chars().for_each(|c| {
            test.on_type(c);
        });

        assert_eq!(
            test.letters_typed(),
            6,
            "should not count untyped letters or extra letters"
        );

        test.on_backspace();

        assert_eq!(
            test.letters_typed(),
            5,
            "should not count untyped letters or extra letters"
        );
    }
}
