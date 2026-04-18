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
        self.quotes.choose(&mut rng).unwrap().clone()
    }

    pub fn get_n_random_words(&self, n: usize) -> Data {
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
