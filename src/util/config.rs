use std::path::PathBuf;
use tokio::fs;
use tokio::sync::mpsc::UnboundedSender;

use serde::{Deserialize, Serialize};

use crate::CustomEvent;
use crate::model::Mode;

use super::toast::{self, ToastMessage};

pub enum ConfigUpdate {
    Mode(Mode),
}

pub struct Config {
    pub data: ConfigData,
    event_tx: UnboundedSender<CustomEvent>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ConfigData {
    #[serde(default)]
    pub mode: Mode,
}

impl Config {
    pub async fn new(event_tx: UnboundedSender<CustomEvent>) -> Config {
        let data = match load().await {
            Ok(data) => data,
            Err(s) => {
                let _ = toast::send(&event_tx, ToastMessage::warning(s));
                ConfigData::default()
            }
        };

        Config { data, event_tx }
    }

    pub async fn handle_config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::Mode(mode) => {
                self.data.mode = mode;
                self.update().await;
            }
        }
    }

    /// Writes to the file
    async fn update(&self) {
        let result: Result<(), String> = async {
            let serialized = toml::to_string(&self.data).map_err(|e| e.to_string())?;
            let path = get_config_path().ok_or_else(|| "Problem getting file path".to_string())?;
            fs::write(path, serialized).await.map_err(|e| e.to_string())
        }
        .await;

        if let Err(e) = result {
            toast::send(
                &self.event_tx,
                ToastMessage::error(format!("Could not update config file: {}", e)),
            )
            .expect("could not send message to toast");
        }
    }
}

/// Try to load the config file from the default path (~/.typing-test-tui.toml)
async fn load() -> color_eyre::Result<ConfigData, String> {
    if let Some(path) = get_config_path() {
        let deserialized = fs::read_to_string(&path).await;
        match deserialized {
            Ok(s) => match toml::from_str::<ConfigData>(&s) {
                Ok(c) => Ok(c),
                Err(e) => Err(format!("Could not deserialize config file. {}", e)),
            },
            Err(e) => Err(format!("Can't read config file, using defaults. {}", e)),
        }
    } else {
        Err("Could not load config path from ~/.typing-test-tui.toml".to_string())
    }
}

pub fn update(
    event_tx: &UnboundedSender<CustomEvent>,
    update: ConfigUpdate,
) -> color_eyre::Result<()> {
    event_tx.send(CustomEvent::ConfigUpdate(update))?;

    Ok(())
}

/// Gets the file as a PathBuf
fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".typing-test-tui.toml"))
}
