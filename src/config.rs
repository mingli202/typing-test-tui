use std::path::PathBuf;
use tokio::{fs, io};

use serde::{Deserialize, Serialize};

use crate::state::Mode;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
}

impl Config {
    pub async fn load() -> Config {
        let path = get_config_path();

        let deserialized = fs::read_to_string(&path).await;
        match deserialized {
            Ok(s) => match toml::from_str::<Config>(&s) {
                Ok(c) => c,
                Err(e) => {
                    println!("Config Error, using defaults. {}", e);
                    Config::default()
                }
            },
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        if let Err(e) = update(Config::default()).await {
                            println!("Can't create default config file. {}", e);
                        };
                    }
                    _ => {
                        println!("Can't read config file, using defaults. {}", e.kind());
                    }
                }
                Config::default()
            }
        }
    }
}

pub fn update_mode(mode: Mode) {
    tokio::spawn(async move {
        let mut config = Config::load().await;
        config.mode = mode;
        update(config).await
    });
}

pub async fn update(config: Config) -> color_eyre::Result<()> {
    let config_file = get_config_path();

    let serialized = toml::to_string(&config)?;
    fs::write(config_file, serialized).await?;

    Ok(())
}

fn get_config_path() -> PathBuf {
    let mut config_path = PathBuf::new();

    if let Some(path) = dirs::home_dir() {
        config_path.push(path);
    }
    config_path.push(".typing-test-tui.toml");

    config_path
}
