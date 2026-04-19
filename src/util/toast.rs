use std::collections::VecDeque;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::macros::text;
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::CustomEvent;

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
    /// The array of messages and their associated timeout task handles
    pub messages: VecDeque<(ToastMessage, JoinHandle<()>)>,

    /// Sender of a ToastMessage
    event_tx: UnboundedSender<CustomEvent>,
}

#[derive(Debug)]
pub enum ToastAction {
    Push(ToastMessage),
    Pop,
}

impl Toast {
    /// A new toast manager with the given sender
    pub fn new(event_tx: UnboundedSender<CustomEvent>) -> Toast {
        Toast {
            messages: VecDeque::new(),
            event_tx,
        }
    }

    /// Convenient method to send message
    pub fn send(&self, msg: ToastMessage) -> color_eyre::Result<()> {
        self.event_tx
            .send(CustomEvent::ToastAction(ToastAction::Push(msg)))?;

        Ok(())
    }

    /// Handle incoming action
    pub fn handle_action(&mut self, action: ToastAction) {
        match action {
            ToastAction::Push(msg) => {
                let event_tx = self.event_tx.clone();
                let handle = tokio::spawn(async move {
                    sleep(Duration::from_secs(3)).await;
                    let _ = event_tx.send(CustomEvent::ToastAction(ToastAction::Pop));
                });

                self.messages.push_front((msg, handle));

                // cap toast length if it ever gets spammed
                // 20 is an arbritary number
                if self.messages.len() > 20 {
                    if let Some((_, handle)) = self.messages.pop_back() {
                        handle.abort();
                    }
                }
            }
            ToastAction::Pop => {
                self.messages.pop_back();
            }
        }
    }
}

/// Convenient method to send message
pub fn send(event_tx: &UnboundedSender<CustomEvent>, msg: ToastMessage) -> color_eyre::Result<()> {
    event_tx.send(CustomEvent::ToastAction(ToastAction::Push(msg)))?;

    Ok(())
}

pub fn view(toast: &Toast, area: Rect, buf: &mut Buffer) {
    let messages = &toast.messages;
    let mut single_toast_area = Rect::new(0, 0, 30, 0);

    single_toast_area.x = area.width.saturating_sub(single_toast_area.width);

    for (message, _) in messages {
        let paragraph =
            Paragraph::new(text![message.msg.clone()].fg(Color::White).bg(Color::Black))
                .black()
                .wrap(Wrap { trim: true })
                .block(
                    Block::bordered()
                        .border_style(message.level.style())
                        .border_type(BorderType::Rounded),
                );

        // calculate height after wrap
        // -2 because it seems it doesn't handle the border
        let line_count = paragraph.line_count(single_toast_area.width - 2);
        single_toast_area.height = line_count as u16;

        paragraph.render(single_toast_area, buf);

        // update y for the next area
        single_toast_area.y += line_count as u16;
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;

    use pretty_assertions::assert_eq;
    use tokio::sync::{Mutex, mpsc};

    #[tokio::test]
    async fn toast_messages() {
        let (tx, mut rx) = mpsc::unbounded_channel::<CustomEvent>();
        let toast = Toast::new(tx);

        let toast = Arc::new(Mutex::new(toast));
        let toast_clone = Arc::clone(&toast);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let CustomEvent::ToastAction(action) = event {
                    let mut lock = toast_clone.lock().await;
                    lock.handle_action(action);
                }
            }
        });

        {
            let lock = toast.lock().await;
            lock.send(ToastMessage::info("First".to_string())).unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        {
            let lock = toast.lock().await;
            lock.send(ToastMessage::warning("Second".to_string()))
                .unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        {
            let lock = toast.lock().await;
            lock.send(ToastMessage::success("Third".to_string()))
                .unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        {
            let lock = toast.lock().await;
            lock.send(ToastMessage::error("Fourth".to_string()))
                .unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        {
            let lock = toast.lock().await;
            let messages: Vec<&ToastMessage> = lock.messages.iter().map(|(msg, _)| msg).collect();
            assert_eq!(
                messages,
                vec![
                    &ToastMessage::error("Fourth".to_string()),
                    &ToastMessage::success("Third".to_string()),
                    &ToastMessage::warning("Second".to_string()),
                    &ToastMessage {
                        level: ToastLevel::Info,
                        msg: "First".to_string()
                    },
                ]
            );
        }

        sleep(Duration::from_millis(3000 - 400)).await;

        {
            let lock = toast.lock().await;
            let messages: Vec<&ToastMessage> = lock.messages.iter().map(|(msg, _)| msg).collect();
            assert_eq!(
                messages,
                vec![
                    &ToastMessage::error("Fourth".to_string()),
                    &ToastMessage::success("Third".to_string()),
                    &ToastMessage::warning("Second".to_string()),
                ]
            );
        }

        sleep(Duration::from_millis(400)).await;

        {
            let lock = toast.lock().await;
            assert_eq!(lock.messages.len(), 0);
        }
    }
}
