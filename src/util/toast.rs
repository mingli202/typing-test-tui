use std::collections::VecDeque;
use std::time::Duration;

use ratatui::style::{Color, Style};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// The possible toast level
/// The only thing this changes is the border color
#[derive(Default, Debug, Clone, PartialEq)]
pub enum ToastLevel {
    #[default]
    Info,
    Warning,
    Error,
    Success,
}

impl ToastLevel {
    /// Gets the border color based on self
    pub fn style(&self) -> Style {
        match self {
            Self::Info => Style::new().white(),
            Self::Warning => Style::new().yellow(),
            Self::Error => Style::new().red(),
            Self::Success => Style::new().green(),
        }
        .bg(Color::Black)
    }
}

/// A singular toast message
#[derive(Default, Debug, PartialEq)]
pub struct ToastMessage {
    pub level: ToastLevel,
    pub msg: String,
}

impl ToastMessage {
    /// New message with info level severity and the given message
    pub fn info(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Info).msg(msg)
    }

    /// New message with warning level severity and the given message
    pub fn warning(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Warning).msg(msg)
    }

    /// New message with error level severity and the given message
    pub fn error(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Error).msg(msg)
    }

    /// New message with success level severity and the given message
    pub fn success(msg: String) -> Self {
        ToastMessage::default().level(ToastLevel::Success).msg(msg)
    }

    /// Returns the messsage with the given level set
    pub fn level(mut self, level: ToastLevel) -> Self {
        self.level = level;
        self
    }

    /// Returns the messsage with the given msg set
    pub fn msg(mut self, msg: String) -> Self {
        self.msg = msg;
        self
    }
}

pub struct Toast {
    /// The array of messages
    pub messages: VecDeque<ToastMessage>,

    /// Receiver of ToastAction
    pub action_rx: UnboundedReceiver<ToastAction>,

    /// Sender of ToastAction
    tx: UnboundedSender<ToastAction>,

    /// Sender of a ToastMessage
    toast_tx: UnboundedSender<ToastMessage>,
}

#[derive(Debug)]
pub enum ToastAction {
    Push(ToastMessage),
    Pop,
}

impl Toast {
    /// A new toast manager with the given sender
    pub fn new(toast_tx: UnboundedSender<ToastMessage>) -> Toast {
        let (tx, rx) = mpsc::unbounded_channel();
        Toast {
            messages: VecDeque::new(),
            action_rx: rx,
            tx,
            toast_tx,
        }
    }

    /// Listens for incoming Toast message and set a timeout to pop it after 3 seconds
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

    /// Convenient method to send message
    pub fn send(&self, msg: ToastMessage) -> color_eyre::Result<()> {
        self.toast_tx.send(msg)?;

        Ok(())
    }

    /// Handle incoming action
    pub fn handle_action(&mut self, action: ToastAction) {
        match action {
            ToastAction::Push(msg) => {
                self.messages.push_front(msg);

                // cap toast length if it ever gets spammed
                // 20 is an arbritary number
                if self.messages.len() > 20 {
                    self.messages.pop_back();
                }
            }
            ToastAction::Pop => {
                self.messages.pop_back();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn toast_messages() {
        let (tx, rx) = mpsc::unbounded_channel::<ToastMessage>();

        let mut toast = Toast::new(tx);
        let handle = toast.init(rx);

        toast.send(ToastMessage::info("First".to_string())).unwrap();

        sleep(Duration::from_millis(100)).await;

        toast
            .send(ToastMessage::warning("Second".to_string()))
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        toast
            .send(ToastMessage::success("Third".to_string()))
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        toast
            .send(ToastMessage::error("Fourth".to_string()))
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        while let Ok(action) = toast.action_rx.try_recv() {
            toast.handle_action(action);
        }

        assert_eq!(
            toast.messages,
            VecDeque::from([
                ToastMessage::error("Fourth".to_string()),
                ToastMessage::success("Third".to_string()),
                ToastMessage::warning("Second".to_string()),
                ToastMessage {
                    level: ToastLevel::Info,
                    msg: "First".to_string()
                },
            ])
        );

        sleep(Duration::from_millis(3000 - 400)).await;

        while let Ok(action) = toast.action_rx.try_recv() {
            toast.handle_action(action);
        }

        assert_eq!(
            toast.messages,
            VecDeque::from([
                ToastMessage::error("Fourth".to_string()),
                ToastMessage::success("Third".to_string()),
                ToastMessage::warning("Second".to_string()),
            ])
        );

        sleep(Duration::from_millis(400)).await;

        while let Ok(action) = toast.action_rx.try_recv() {
            toast.handle_action(action);
        }

        assert_eq!(toast.messages.len(), 0);

        drop(toast);

        handle.await.unwrap();
    }
}
