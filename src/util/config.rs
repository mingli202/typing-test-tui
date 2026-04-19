use std::path::PathBuf;
use tokio::fs;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use serde::{Deserialize, Serialize};

use crate::CustomEvent;
use crate::model::Mode;

use super::toast::{self, ToastMessage};

pub enum ConfigUpdate {
    Mode(Mode),
}

pub struct Config {
    pub data: ConfigData,
    config_tx: UnboundedSender<ConfigData>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
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

        let (config_tx, config_rx) = mpsc::unbounded_channel();

        init_config_loop(config_rx, event_tx);

        Config { data, config_tx }
    }

    pub fn handle_config_update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::Mode(mode) => {
                self.data.mode = mode;
                let _ = self.config_tx.send(self.data.clone());
            }
        }
    }
}

fn init_config_loop(
    mut config_rx: UnboundedReceiver<ConfigData>,
    event_tx: UnboundedSender<CustomEvent>,
) {
    tokio::spawn(async move {
        while let Some(data) = config_rx.recv().await {
            let result: Result<(), String> = async {
                let serialized = toml::to_string(&data).map_err(|e| e.to_string())?;
                let path =
                    get_config_path().ok_or_else(|| "Problem getting file path".to_string())?;
                fs::write(path, serialized).await.map_err(|e| e.to_string())
            }
            .await;

            if let Err(e) = result {
                toast::send(
                    &event_tx,
                    ToastMessage::error(format!("Could not update config file: {}", e)),
                )
                .expect("could not send message to toast");
            }
        }
    });
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

/// Gets the file as a PathBuf
fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".typing-test-tui.toml"))
}
