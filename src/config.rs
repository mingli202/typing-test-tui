use std::path::PathBuf;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::{fs, io};

use serde::{Deserialize, Serialize};

use crate::state::Mode;
use crate::toast::ToastMessage;

pub enum ConfigUpdate {
    Mode(Mode),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
}

impl Config {
    pub fn init(
        mut rx: UnboundedReceiver<ConfigUpdate>,
        toast_tx: UnboundedSender<ToastMessage>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(mut update) = rx.recv().await {
                while let Ok(newer) = rx.try_recv() {
                    update = newer;
                }

                match update {
                    ConfigUpdate::Mode(mode) => match Config::load().await {
                        Ok(config) => {
                            if let Err(err) = config.mode(mode).update().await {
                                toast_tx
                                    .send(ToastMessage::error(format!(
                                        "Could not update config. {}",
                                        err
                                    )))
                                    .expect("could not send message to toast");
                            }
                        }
                        Err(e) => {
                            toast_tx
                                .send(ToastMessage::error(format!("Update error, {}", e)))
                                .expect("could not send message to toast");
                        }
                    },
                };
            }
        })
    }

    pub async fn load() -> color_eyre::Result<Config, String> {
        if let Some(path) = get_config_path() {
            let deserialized = fs::read_to_string(&path).await;
            match deserialized {
                Ok(s) => match toml::from_str::<Config>(&s) {
                    Ok(c) => Ok(c),
                    Err(e) => Err(format!("Could not deserialize config file. {}", e)),
                },
                Err(e) => {
                    let reason = match e.kind() {
                        io::ErrorKind::NotFound => {
                            if let Err(e) = Config::default().update().await {
                                format!(
                                    "Could not create default config file at {}. {}",
                                    path.display(),
                                    e
                                )
                            } else {
                                format!(
                                    "Could not find config file at {}, using default.",
                                    path.display()
                                )
                            }
                        }
                        _ => {
                            format!("Can't read config file, using defaults. {}", e)
                        }
                    };

                    Err(reason)
                }
            }
        } else {
            Err("Could not load config path from ~/.typing-test-tui.toml".to_string())
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

fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".typing-test-tui.toml"))
}
