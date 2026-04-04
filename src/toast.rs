use std::collections::VecDeque;
use std::time::Duration;

use ratatui::style::Style;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;

#[derive(Default, Debug)]
pub enum ToastLevel {
    #[default]
    Info,
    Warning,
    Error,
    Success,
}

impl ToastLevel {
    pub fn style(&self) -> Style {
        match self {
            Self::Info => Style::new().white(),
            Self::Warning => Style::new().yellow(),
            Self::Error => Style::new().red(),
            Self::Success => Style::new().green(),
        }
    }
}

#[derive(Default, Debug)]
pub struct ToastMessage {
    pub level: ToastLevel,
    pub msg: String,
}

impl ToastMessage {
    pub fn info(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Info).msg(msg)
    }
    pub fn warning(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Warning).msg(msg)
    }
    pub fn error(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Error).msg(msg)
    }
    pub fn success(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Success).msg(msg)
    }

    pub fn level(mut self, level: ToastLevel) -> Self {
        self.level = level;
        self
    }

    pub fn msg(mut self, msg: String) -> Self {
        self.msg = msg;
        self
    }
}

pub struct Toast {
    pub messages: VecDeque<ToastMessage>,
    pub rx: UnboundedReceiver<ToastAction>,
    tx: UnboundedSender<ToastAction>,
    toast_tx: UnboundedSender<ToastMessage>,
}

#[derive(Debug)]
pub enum ToastAction {
    Push(ToastMessage),
    Pop,
}

impl Toast {
    pub fn new(toast_tx: UnboundedSender<ToastMessage>) -> Toast {
        let (tx, rx) = mpsc::unbounded_channel();
        Toast {
            messages: VecDeque::new(),
            rx,
            tx,
            toast_tx,
        }
    }

    pub fn init(&self, mut toast_rx: UnboundedReceiver<ToastMessage>) -> JoinHandle<()> {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = toast_rx.recv().await {
                let _ = tx.clone().send(ToastAction::Push(msg));

                let tx = tx.clone();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(3)).await;
                    let _ = tx.send(ToastAction::Pop);
                });
            }
        })
    }

    pub fn send(&self, msg: ToastMessage) -> color_eyre::Result<()> {
        self.toast_tx.send(msg)?;

        Ok(())
    }
}
