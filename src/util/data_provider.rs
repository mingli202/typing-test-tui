use std::collections::HashMap;
use std::fs;

use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::model::Mode;

#[derive(Clone, Debug)]
pub struct Data {
    pub text: String,
    pub source: String,
}

pub struct DataProvider {
    words: Vec<String>,
    quotes: Vec<Data>,
}

impl DataProvider {
    pub fn new(
        words_path: Option<String>,
        quotes_path: Option<String>,
    ) -> color_eyre::Result<Self> {
        let words = get_words(words_path)?;
        let quotes = get_quotes(quotes_path)?;

        Ok(DataProvider { words, quotes })
    }

    pub fn get_data_from_mode(&self, mode: &Mode) -> Data {
        match mode {
            Mode::Quote => self.get_random_quote(),
            Mode::Words(n) => self.get_n_random_words(*n),
            // TODO: new lines as the user reaches the end
            // max 80 char per line -> ~16 words
            // preload 4 lines
            //
            // NOTE: require refactor of current architecture or it will become messy
            // for now, just assume the user won't type more than 240 wpm
            Mode::Time(t) => {
                let mut data = self.get_n_random_words(t * 4);
                data.source = format!("{} seconds", t);
                data
            }
        }
    }

    pub fn get_random_quote(&self) -> Data {
        let mut rng = rand::rng();
        self.quotes.choose(&mut rng).map_or_else(
            || Data {
                text: "No quotes available".to_string(),
                source: "no quotes available".to_string(),
            },
            |data| data.clone(),
        )
    }

    pub fn get_n_random_words(&self, n: usize) -> Data {
        if self.words.is_empty() {
            return Data {
                text: "No words found".to_string(),
                source: "No words found".to_string(),
            };
        }

        if self.words.len() == 1 {
            let word = self.words[0].clone();
            return Data {
                text: vec![word; n].join(" "),
                source: format!("{} words", n),
            };
        }

        let mut rng = rand::rng();

        let mut v = Vec::with_capacity(n);

        let mut last = -1;
        let mut ind = -1;

        let words = &self.words;

        for _ in 0..n {
            while ind == last {
                ind = rng.random_range(0..words.len()) as i32;
            }

            v.push(words[ind as usize].clone());

            last = ind;
        }

        Data {
            text: v.join(" "),
            source: format!("{} words", n),
        }
    }
}

/// Gets all the words from the given path if Some, otherwise default to built-in words
fn get_words(path: Option<String>) -> color_eyre::Result<Vec<String>> {
    let json = if let Some(path) = path {
        &fs::read_to_string(path)?
    } else {
        include_str!("../../assets/english.json")
    };

    let data = serde_json::from_str::<Vec<String>>(json)?;

    Ok(data)
}

/// Gets all the quotes from the given path if Some, otherwise default to built-in quotes
fn get_quotes(path: Option<String>) -> color_eyre::Result<Vec<Data>> {
    let json = if let Some(path) = path {
        &fs::read_to_string(path)?
    } else {
        include_str!("../../assets/quotes.json")
    };

    let data = serde_json::from_str::<HashMap<String, Vec<String>>>(json)?;

    Ok(data
        .into_iter()
        .flat_map(|(src, qs)| {
            let mut qs = qs;
            let mut v = vec![];

            while let Some(quote) = qs.pop() {
                if quote != src {
                    v.push(Data {
                        source: src.clone(),
                        text: quote,
                    });
                }
            }
            v
        })
        .filter(|q| !q.text.is_empty())
        .collect())
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "typing_test_tui_data_provider_{name}_{}_{}.json",
            std::process::id(),
            nanos
        ))
    }

    fn write_temp_file(name: &str, contents: &str) -> String {
        let path = temp_file_path(name);
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn get_n_random_words_returns_fallback_when_words_are_empty() {
        let provider = DataProvider {
            words: vec![],
            quotes: vec![],
        };

        let data = provider.get_n_random_words(5);

        assert_eq!(
            data.text,
            "No words found",
            "empty datasets should use the fallback message"
        );
        assert_eq!(data.source, "No words found");
    }

    #[test]
    fn get_n_random_words_with_single_word_repeats_for_requested_count() {
        let provider = DataProvider {
            words: vec!["hello".to_string()],
            quotes: vec![],
        };

        let data = provider.get_n_random_words(3);

        assert_eq!(data.text, "hello hello hello");
        assert_eq!(data.source, "3 words");
    }

    #[test]
    fn get_n_random_words_with_zero_count_returns_empty_text() {
        let provider = DataProvider {
            words: vec!["hello".to_string()],
            quotes: vec![],
        };

        let data = provider.get_n_random_words(0);

        assert_eq!(data.text, "");
        assert_eq!(data.source, "0 words");
    }

    #[test]
    fn get_random_quote_returns_fallback_when_quotes_are_empty() {
        let provider = DataProvider {
            words: vec![],
            quotes: vec![],
        };

        let data = provider.get_random_quote();

        assert_eq!(data.text, "No quotes available");
        assert_eq!(data.source, "no quotes available");
    }

    #[test]
    fn get_data_from_time_mode_uses_word_count_and_seconds_source() {
        let provider = DataProvider {
            words: vec!["hello".to_string()],
            quotes: vec![],
        };

        let data = provider.get_data_from_mode(&Mode::Time(2));

        assert_eq!(data.text, "hello hello hello hello hello hello hello hello");
        assert_eq!(data.source, "2 seconds");
    }

    #[test]
    fn get_words_reads_custom_json_file() {
        let path = write_temp_file("words", r#"["alpha","beta"]"#);

        let words = get_words(Some(path.clone())).unwrap();

        assert_eq!(words, vec!["alpha".to_string(), "beta".to_string()]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn get_words_returns_error_for_invalid_json() {
        let path = write_temp_file("invalid_words", r#"{"not":"an array"}"#);

        let result = get_words(Some(path.clone()));

        assert!(result.is_err(), "invalid word JSON should fail to parse");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn get_quotes_filters_empty_and_source_matching_entries() {
        let path = write_temp_file(
            "quotes",
            r#"{
                "Author A": ["Author A", "", "Keep going"],
                "Author B": ["Stay focused"]
            }"#,
        );

        let mut quotes = get_quotes(Some(path.clone())).unwrap();
        quotes.sort_by(|a, b| a.source.cmp(&b.source).then(a.text.cmp(&b.text)));

        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].source, "Author A");
        assert_eq!(quotes[0].text, "Keep going");
        assert_eq!(quotes[1].source, "Author B");
        assert_eq!(quotes[1].text, "Stay focused");

        fs::remove_file(path).unwrap();
    }

    #[test]
    #[should_panic]
    fn get_data_from_time_mode_panics_on_overflow() {
        let provider = DataProvider {
            words: vec!["hello".to_string()],
            quotes: vec![],
        };

        let _ = provider.get_data_from_mode(&Mode::Time(usize::MAX));
    }
}
