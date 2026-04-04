use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio::{fs, io};

use serde::{Deserialize, Serialize};

use crate::state::Mode;

pub enum ConfigUpdate {
    Mode(Mode),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
}

impl Config {
    pub fn init(mut rx: UnboundedReceiver<ConfigUpdate>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(mut update) = rx.recv().await {
                while let Ok(newer) = rx.try_recv() {
                    update = newer;
                }

                match update {
                    ConfigUpdate::Mode(mode) => {
                        if let Err(err) = update_mode(mode).await {
                            eprintln!("could not update config {}", err);
                        }
                    }
                };
            }
        })
    }

    pub async fn load() -> Config {
        if let Some(path) = get_config_path() {
            let deserialized = fs::read_to_string(&path).await;
            match deserialized {
                Ok(s) => match toml::from_str::<Config>(&s) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Config Error, using defaults. {}", e);
                        Config::default()
                    }
                },
                Err(e) => {
                    match e.kind() {
                        io::ErrorKind::NotFound => {
                            if let Err(e) = Config::default().update().await {
                                eprintln!("Can't create default config file. {}", e);
                            };
                        }
                        _ => {
                            eprintln!("Can't read config file, using defaults. {}", e.kind());
                        }
                    }
                    Config::default()
                }
            }
        } else {
            eprintln!("Could not load config path");
            Config::default()
        }
    }

    fn mode(mut self, mode: Mode) -> Config {
        self.mode = mode;
        self
    }

    async fn update(self) -> color_eyre::Result<()> {
        if let Some(file) = get_config_path() {
            let serialized = toml::to_string(&self)?;
            fs::write(file, serialized).await?;
        }
        Ok(())
    }
}

async fn update_mode(mode: Mode) -> color_eyre::Result<()> {
    Config::load().await.mode(mode).update().await
}

fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".typing-test-tui.toml"))
}
