use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    #[serde(skip)]
    pub source: String,
    #[serde(skip)]
    pub quote: String,
}

#[derive(Debug, Default)]
pub struct Data {
    pub text: String,
    pub source: String,
}

impl Data {
    pub fn get_random_quote() -> Data {
        let mut rng = rand::rng();
        let quotes = Data::get_quotes();
        let Quote { source, quote } = quotes.choose(&mut rng).unwrap();
        Data {
            source: source.clone(),
            text: quote.clone(),
        }
    }

    pub fn get_n_random_words(n: usize) -> Data {
        let mut rng = rand::rng();

        let mut v = Vec::with_capacity(n);

        let mut last = -1;
        let mut ind = -1;

        let words = Data::get_words();

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

    fn get_words() -> Vec<String> {
        serde_json::from_str::<Vec<String>>(include_str!("../../assets/english.json")).unwrap()
    }

    fn get_quotes() -> Vec<Quote> {
        serde_json::from_str::<HashMap<String, Vec<String>>>(include_str!(
            "../../assets/quotes.json"
        ))
        .unwrap()
        .into_iter()
        .flat_map(|(src, qs)| {
            let mut qs = qs;
            let mut v = vec![];

            while let Some(quote) = qs.pop() {
                if quote != src {
                    v.push(Quote {
                        source: src.clone(),
                        quote,
                    });
                }
            }
            v
        })
        .filter(|q| !q.quote.is_empty())
        .collect()
    }
}

// #[allow(unused, non_snake_case)]
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     use std::collections::HashSet;
//
//     use serde::Deserialize;
//     use tokio::task::JoinSet;
//
//     #[derive(Deserialize, Debug)]
//     struct Phonetic {
//         text: Option<String>,
//         audio: Option<String>,
//         sourceUrl: Option<String>,
//         license: Option<License>,
//     }
//
//     #[derive(Deserialize, Debug)]
//     struct License {
//         name: String,
//         url: String,
//     }
//
//     #[derive(Deserialize, Debug)]
//     struct Definition {
//         definition: String,
//         example: Option<String>,
//         synonyms: Vec<String>,
//         antonyms: Vec<String>,
//     }
//
//     #[derive(Deserialize, Debug)]
//     struct Meaning {
//         partOfSpeech: String,
//         definitions: Vec<Definition>,
//         synonyms: Vec<String>,
//         antonyms: Vec<String>,
//     }
//
//     #[derive(Deserialize, Debug)]
//     struct Res {
//         word: String,
//         phonetic: Option<String>,
//         phonetics: Vec<Phonetic>,
//         origin: Option<String>,
//         meanings: Vec<Meaning>,
//         license: Option<License>,
//         sourceUrls: Vec<String>,
//     }
//
//     #[derive(Deserialize, Debug)]
//     struct NotFound {
//         title: String,
//         message: String,
//         resolution: String,
//     }
//
//     pub async fn exists(word: String) -> Option<bool> {
//         let re = reqwest::get(format!(
//             "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
//             word
//         ))
//         .await;
//
//         if re.is_err() {
//             return None;
//         }
//
//         let re = re.unwrap().text().await;
//
//         if re.is_err() {
//             return None;
//         }
//
//         let txt = re.unwrap();
//
//         if txt.contains("1015") {
//             return None;
//         }
//
//         match serde_json::from_str::<NotFound>(&txt) {
//             Ok(_) => Some(false),
//             Err(_) => Some(true),
//         }
//     }
//     #[test]
//     fn format() {
//         let quotes = Data::get_quotes();
//
//         for quote in quotes {
//             assert!(
//                 quote.source.split(' ').all(|s| !s.is_empty()),
//                 "there are empty words"
//             );
//             assert!(
//                 quote.quote.split(' ').all(|s| !s.is_empty()),
//                 "there are empty words"
//             );
//         }
//     }
//
//     use std::sync::Arc;
//     use tokio::sync::Mutex;
//
//     use tokio::{sync::mpsc, task};
//
//     #[tokio::test(flavor = "multi_thread")]
//     async fn chect_grammar() {
//         return;
//         let ignore_words: HashSet<&str> = HashSet::from([
//             "enlightenment,",
//             "book;",
//             "read.",
//             "confident,",
//             "pain,",
//             "believe.",
//             "bias.",
//             "are.",
//             "equal,",
//             "hip.",
//             "fortune.",
//             "complexity,",
//             "something,",
//             "fearful,",
//             "way,",
//             "hostilities.",
//             "'myths'",
//             "microexpression",
//             "starving,",
//             "how.",
//             "I've",
//             "games,",
//             "attack,",
//             "years.",
//             "stress;",
//             "realm,",
//             "within,",
//             "hostility,",
//             "mathematicians'",
//             "force.",
//             "I'm",
//             "grows,",
//             "yourselves.",
//             "phenomena,",
//             "promotion,",
//             "mother.",
//             "terrain,",
//             "care,",
//             "destiny.",
//             "life's",
//             "lives",
//             "positive,",
//             "person's",
//             "self-affirmation,",
//             "clear.",
//             "emotion,",
//             "inflections,",
//             "you.",
//             "event.",
//             "say.",
//             "cultivate,",
//             "\"Why",
//             "quickly,",
//             "motives,",
//             "narcissists,",
//             "surface,",
//             "lure.",
//             "\"Write",
//             "attention,",
//             "we're",
//             "him.",
//             "action,",
//             "society.",
//             "differently.",
//             "learn.",
//             "it!",
//             "ridiculed.",
//             "mark.",
//             "another.",
//             "hand,",
//             "ship's",
//             "built,",
//             "coming,",
//             "idea,",
//             "lightning-bolt",
//             "pilot.",
//             "so,",
//             "strain.",
//             "friend;",
//             "uniqueness,",
//             "away.",
//             "thought,",
//             "him;",
//             "fate.",
//             "to.",
//             "humble,",
//             "tower,",
//             "exist.",
//             "2",
//             "figures,",
//             "Yoo",
//             "deterred.",
//             "hurt,",
//             "vaping.\"",
//             "choices,",
//             "envy.",
//             "Story,",
//             "greed,",
//             "angered,",
//             "superiority.",
//             "wars,",
//             "groups.",
//             "day.",
//             "say,",
//             "intriguing.",
//             "purpose.",
//             "polite,",
//             "it's",
//             "speed.",
//             "attacked.",
//             "exists,",
//             "Finally,",
//             "That's",
//             "lost,",
//             "irrationally,",
//             "come,",
//             "around.",
//             "cues",
//             "night,",
//             "better.",
//             "disappeared,",
//             "corncribs,",
//             "adolescence,",
//             "pride!",
//             "reject.",
//             "wall.\"",
//             "active,",
//             "cunning;",
//             "inhabits.",
//             "diligent.",
//             "failing,",
//             "communication.",
//             "mourning.",
//             "resiliency.",
//             "goals.",
//             "effects,",
//             "play.",
//             "'em,",
//             "dark,",
//             "was",
//             "term.",
//             "Dora,",
//             "greatness.",
//             "trust,",
//             "overstimulated;",
//             "differently,",
//             "fantasy.",
//             "headlines.",
//             "desire.",
//             "am,",
//             "uses.",
//             "pawns.",
//             "Look",
//             "patterns.",
//             "pranks.",
//             "free;",
//             "group.",
//             "this,",
//             "variety.",
//             "Abbott",
//             "weakened;",
//             "moment,",
//             "proud,",
//             "ambivalent,",
//             "potential.",
//             "war-horses",
//             "before.",
//             "origins,",
//             "\"The",
//             "tight",
//             "exigent,",
//             "love,",
//             "self-absorption.",
//             "powerful,",
//             "disadvantages,",
//             "last.",
//             "chances.",
//             "other.",
//             "desires,",
//             "We,",
//             "afraid,",
//             "relationship,",
//             "seems,",
//             "well,",
//             "obvious.",
//             "person.",
//             "Evil',",
//             "situation.",
//             "view.",
//             "scrutiny,",
//             "others,",
//             "expressions,",
//             "touch.",
//             "appeal.",
//             "cortex,",
//             "incompetent.",
//             "battle-tested.",
//             "will.",
//             "saintliness,",
//             "entertainment,",
//             "strategy,",
//             "measures.",
//             "virtue;",
//             "limits.",
//             "thirsty.",
//             "plough,",
//             "work,",
//             "following:",
//             "superior.",
//             "us,",
//             "Yoosung's",
//             "however,",
//             "powers.",
//             "ego.",
//             "laughed.",
//             "That...is",
//             "me.",
//             "Elkia",
//             "have,",
//             "second.",
//             "flat.",
//             "proportion;",
//             "answer.",
//             "perspective,",
//             "civilized.",
//             "general.",
//             "step.",
//             "worst,",
//             "disposal,",
//             "unavoidable,",
//             "desperation.",
//             "open,",
//             "end,",
//             "naturally;",
//             "talk.",
//             "mood,",
//             "role,",
//             "reading,",
//             "looked",
//             "horse's",
//             "exaltation.",
//             "day,",
//             "varieties,",
//             "Now,",
//             "toughness,",
//             "didn't",
//             "passed.",
//             "ourselves.",
//             "do,",
//             "who,",
//             "-",
//             "sabotage.",
//             "impression.",
//             "is",
//             "do!",
//             "it:",
//             "job,",
//             "not,",
//             "simple.",
//             "mountains.",
//             "attack.",
//             "faster,",
//             "ways,",
//             "easy,",
//             "Olympus's",
//             "direction.",
//             "positions,",
//             "events,",
//             "changed.",
//             "suffering.",
//             "changes.",
//             "Our",
//             "soundlessness.",
//             "formation,",
//             "feeling.",
//             "adversity,",
//             "race!",
//             "back!",
//             "achievers.",
//             "ourselves,",
//             "insecurities.",
//             "compulsion.",
//             "society,",
//             "bosses,",
//             "emotions,",
//             "impossible.",
//             "them.",
//             "moments,",
//             "circumstances,",
//             "unemotional,",
//             "discontent,",
//             "by,",
//             "types.",
//             "attitude.",
//             "sex.",
//             "long.",
//             "shadow,",
//             "leader,",
//             "cetera",
//             "this.",
//             "character,",
//             "personality,",
//             "trends,",
//             "For",
//             "enviers",
//             "eighteenth-century",
//             "citizens.",
//             "sky,",
//             "point.",
//             "lightbulb,",
//             "mockingbird.",
//             "thinking.",
//             "genetic,",
//             "Be",
//             "fantasies.",
//             "cultured,",
//             "once.",
//             "However,",
//             "reserved,",
//             "ass.",
//             "strength.",
//             "position.",
//             "omen,",
//             "left.",
//             "afraid.",
//             "angry,",
//             "full.",
//             "presence.",
//             "timing,",
//             "people.",
//             "Weakness'",
//             "tendencies.",
//             "It's",
//             "help.",
//             "achievements,",
//             "interest,",
//             "biases.",
//             "others.",
//             "risks,",
//             "Kim",
//             "humans.",
//             "release,",
//             "lonely.",
//             "levels.",
//             "mode.",
//             "dangers,",
//             "anyway...\"",
//             "better,",
//             "status,",
//             "body,",
//             "decaying,",
//             "relativity.",
//             "changing.",
//             "energy,",
//             "\"At",
//             "comprehended,",
//             "culture,",
//             "phase,",
//             "valleys,",
//             "attacking,",
//             "humor;",
//             "it,",
//             "control,",
//             "stake,",
//             "competitive.",
//             "habits.",
//             "strong!",
//             "straight.",
//             "intuitive.",
//             "courage,",
//             "react,",
//             "fact,",
//             "dream.",
//             "fence.",
//             "done,",
//             "People's",
//             "endowment.",
//             "gardens,",
//             "frustrated,",
//             "game,",
//             "conscience.",
//             "interests.",
//             "himself.",
//             "unified,",
//             "again.",
//             "gentle,",
//             "imagination,",
//             "servitude.",
//             "land.",
//             "wood,",
//             "remained.",
//             "unknowable,",
//             "receive,",
//             "Dokja",
//             "comrades,",
//             "poison.",
//             "with,",
//             "experiments,",
//             "breathing.",
//             "win.",
//             "ambitions.",
//             "response,",
//             "sorrow.",
//             "too,",
//             "mourning:",
//             "actions.",
//             "life,",
//             "success,",
//             "grandiosity,",
//             "lead,",
//             "puzzles.",
//             "anything.",
//             "happening,",
//             "enchanting,",
//             "\"I...\"",
//             "slanderers",
//             "producing.",
//             "unknowable.",
//             "favor.",
//             "Zeus's",
//             "mysterious,",
//             "attitude,",
//             "Edison",
//             "war,",
//             "respect.",
//             "be?",
//             "them;",
//             "ends.",
//             "pain.",
//             "which.",
//             "doing.",
//             "direction,",
//             "counterstrike!",
//             "for",
//             "trust.",
//             "advantages.",
//             "coins.",
//             "setback.",
//             "interesting.",
//             "weak.",
//             "awesome.",
//             "goal,",
//             "work.",
//             "decisions.",
//             "reality,",
//             "desperate.",
//             "are,",
//             "story'",
//             "\"...What",
//             "frustrations.",
//             "Yoosung",
//             "determined,",
//             "ignorance.",
//             "can.",
//             "Thomas",
//             "shouldn't",
//             "become.",
//             "gestures.",
//             "you,",
//             "knew.",
//             "grandiose.",
//             "curviness",
//             "outward,",
//             "value.",
//             "science.",
//             "doubt,",
//             "reflect,",
//             "sacrifices;",
//             "benefit.",
//             "it.",
//             "feel.",
//             "oneself.",
//             "love,'",
//             "members,",
//             "most.",
//             "contempt.",
//             "actors,",
//             "plans;",
//             "reality.",
//             "sense.",
//             "greedy,",
//             "order,",
//             "greatness,",
//             "remains:",
//             "defeated.",
//             "necessity,",
//             "look",
//             "spoken.",
//             "relax.",
//             "effective,",
//             "wings.",
//             "power,",
//             "well.",
//             "transcendental.",
//             "character.",
//             "above.",
//             "circumstance.",
//             "Realm.",
//             "monster.",
//             "rabbits.",
//             "every",
//             "himself,",
//             "words.",
//             "despair.",
//             "your",
//             "victories,",
//             "(feedback)",
//             "can't",
//             "independent,",
//             "Come,",
//             "planet.",
//             "tried-and-true",
//             "us.",
//             "justice.",
//             "opponent's",
//             "higher,",
//             "water.",
//             "quality.",
//             "haughty.",
//             "were...the",
//             "qualities,",
//             "things,",
//             "arts,",
//             "see.",
//             "comparisons.",
//             "wins,",
//             "times,",
//             "emptiness.",
//             "competent,",
//             "wall,",
//             "difficult,",
//             "mask,",
//             "there.",
//             "No,",
//             "new.",
//             "win,",
//             "world!",
//             "guests,",
//             "Tzu,",
//             "interest.",
//             "empathy,",
//             "malleable.",
//             "birth,",
//             "seems.",
//             "chance,",
//             "wounds,",
//             "influence,",
//             "schadenfreude.",
//             "bidding.",
//             "up.",
//             "see,",
//             "\"I",
//             "battles.",
//             "circumstances.",
//             "used,",
//             "identity.",
//             "man,",
//             "cry.",
//             "vision;",
//             "individual's",
//             "Indeed,",
//             "\"Sir?\"",
//             "be",
//             "out.",
//             "will!",
//             "respect,",
//             "weak,",
//             "subtle,",
//             "world,",
//             "level.",
//             "luck,",
//             "strength,",
//             "criticized;",
//             "mansions.",
//             "succession,",
//             "rationality.",
//             "urged,",
//             "authority.",
//             "money.",
//             "possessiveness.",
//             "'Stay",
//             "You're",
//             "leadership,",
//             "milk,",
//             "leadership.",
//             "world.",
//             "overidentifying",
//             "visible.",
//             "weren't",
//             "quo.",
//             "intelligence,",
//             "Your",
//             "ocean.",
//             "one's",
//             "lacking,",
//             "Hakuin",
//             "equal.",
//             "resistant.",
//             "intelligence.",
//             "\"You",
//             "is.",
//             "Weapons,",
//             "setting,",
//             "naturally.",
//             "surprise.",
//             "Zen,",
//             "die.",
//             "discipline.",
//             "experience,",
//             "want,",
//             "countryside.",
//             "uncarved",
//             "fate,",
//             "begin!",
//             "years,",
//             "small.",
//             "beautiful,",
//             "\"John",
//             "aloof.",
//             "doesn't",
//             "trait;",
//             "suspicious,",
//             "'new",
//             "unconscious.",
//             "resistance,",
//             "them:",
//             "prank,",
//             "ineffective.",
//             "long-term.",
//             "follows:",
//             "Stream,",
//             "sprouts,",
//             "Albert",
//             "peoples,",
//             "resistance.",
//             "self-opinion",
//             "scheme,",
//             "Instead,",
//             "Einstein",
//             "projections,",
//             "energy.",
//             "great.",
//             "smile,",
//             "narrative:",
//             "victory.",
//             "view-\"",
//             "solution,",
//             "couldn't",
//             "power.",
//             "pull.",
//             "yourself,",
//             "level,",
//             "our",
//             "nature.",
//             "objective.",
//             "strong,",
//             "wonder.",
//             "everything.",
//             "Every",
//             "defend.",
//             "instrument,",
//             "adapt.",
//             "profession.",
//             "gonna",
//             "Immanities.",
//             "empathy.",
//             "mind:",
//             "forbidden.",
//             "radium.",
//             "dramatic.",
//             "enough!",
//             "age.",
//             "behavior,",
//             "lives,",
//             "Marie",
//             "finances.",
//             "story.",
//             "inclinations.",
//             "eggs,",
//             "hurt.\"",
//             "whole,",
//             "psychology.",
//             "genius.",
//             "yourself.",
//             "disappointment.",
//             "wisdom,",
//             "'Absolute",
//             "accomplishments.",
//             "plants.",
//             "legitimacy,",
//             "side.",
//             "interdependently.",
//             "effort.",
//             "themselves,",
//             "powerful.",
//             "realistic,",
//             "painful.",
//             "far.",
//             "contract.",
//             "like.",
//             "way.",
//             "categorize,",
//             "challenges.",
//             "justice;",
//             "aggressive,",
//             "all.",
//             "half.",
//             "weapons;",
//             "ago.",
//             "out,",
//             "animals,",
//             "swim.",
//             "bring,",
//             "possess.",
//             "endeavors,",
//             "them,",
//             "curve.",
//             "difference.",
//             "oppression.",
//             "sublime,",
//             "decisions,",
//             "life.",
//             "When,",
//             "winter;",
//             "truth,",
//             "autonomous.",
//             "more.",
//             "time,",
//             "impossible...that's",
//             "parties,",
//             "suffocates;",
//             "inward.",
//             "success.",
//             "thousands",
//             "hurtful.",
//             "Joonghyuk's",
//             "functional,",
//             "alone.",
//             "inferiority,",
//             "norms.",
//             "excitement,",
//             "meaning,",
//             "it;",
//             "groups,",
//             "group,",
//             "too.",
//             "people's",
//             "lives.",
//             "moment!",
//             "war.",
//             "like,",
//             "incarnations.",
//             "not.",
//             "injuries.",
//             "stories.",
//             "Fortunately,",
//             "defense,",
//             "against,",
//             "women.",
//             "their",
//             "touchiness",
//             "example,",
//             "rule.",
//             "front.",
//             "imagine.",
//             "Rockefeller",
//             "midst.",
//             "vaping.",
//             "egos,",
//             "enemy.",
//             "events.",
//             "monster's",
//             "alone,",
//             "enjoy.",
//             "service.",
//             "itself,",
//             "ideas.",
//             "listen.\"",
//             "be.",
//             "purpose,",
//             "nothing,",
//             "price.",
//             "shape:",
//             "themselves.",
//             "rigid.",
//             "knowledge,",
//             "fields.",
//             "tamed.",
//             "truth.",
//             "Sublime.",
//             "Hee-hee,",
//             "hyung",
//             "sides:",
//             "return,",
//             "weather,",
//             "fight,",
//             "technique:",
//             "formlessness.",
//             "have.",
//             "defeat.",
//             "won't",
//             "process.",
//             "pig,",
//             "time.",
//             "self-belief",
//         ]);
//
//         let quotes = Data::get_quotes();
//
//         let all_words = quotes
//             .iter()
//             .flat_map(|q| {
//                 q.quote
//                     .split(&[' ', '.', ','])
//                     .map(|s| s.to_string().to_lowercase())
//                     .filter(|s| !s.is_empty())
//             })
//             .collect::<HashSet<_>>();
//
//         println!("All words: {}", all_words.len());
//
//         let mut set = JoinSet::new();
//
//         let count = Arc::new(Mutex::new(0));
//
//         for (i, word) in all_words.into_iter().enumerate() {
//             if ignore_words.contains(&word[..]) {
//                 continue;
//             }
//
//             // println!("Word: {:#?}", word);
//             let count = Arc::clone(&count);
//
//             set.spawn(async move {
//                 let mut res = exists(word.to_string()).await;
//
//                 while res.is_none() {
//                     // println!("some problem");
//                     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//                     res = exists(word.to_string()).await;
//                 }
//
//                 let mut count = count.lock().await;
//                 *count += 1;
//                 println!("Word {}: {} ({})", count, word, i);
//
//                 if let Some(false) = res {
//                     Some(word)
//                 } else {
//                     None
//                 }
//             });
//         }
//
//         let errors = set
//             .join_all()
//             .await
//             .into_iter()
//             .flatten()
//             .collect::<Vec<_>>();
//
//         let empty_vec: Vec<String> = vec![];
//         assert_eq!(errors, empty_vec);
//     }
//
//     #[test]
//     fn random_words_and_quotes() {
//         let random_words = Data::get_n_random_words(10);
//
//         assert_eq!(10, random_words.text.split(" ").count());
//
//         let mut last = String::new();
//
//         let random_words = random_words.text.split(" ").collect::<Vec<&str>>();
//
//         for word in random_words {
//             if last == word {
//                 panic!("Repeating word");
//             }
//             last = word.to_string()
//         }
//     }
// }
