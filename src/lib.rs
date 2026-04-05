use std::time::Duration;

use ratatui::crossterm::event::{self, KeyCode};
use ratatui::layout::Rect;
use ratatui::macros::text;
use ratatui::style::{Color, Stylize};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::mpsc::UnboundedSender;

use self::config::{Config, ConfigUpdate};
use self::state::{Action, State};
use self::toast::{Toast, ToastMessage};

pub mod config;
pub mod data;
mod state;
pub mod toast;
mod typing_test;

pub struct App {
    state: State,
    exit: bool,
    config_tx: UnboundedSender<ConfigUpdate>,
    toast: Toast,
}

impl App {
    pub async fn new(tx: UnboundedSender<ConfigUpdate>, toast: Toast) -> Self {
        let config = match Config::load().await {
            Ok(config) => config,
            Err(e) => {
                let _ = toast.send(ToastMessage::error(e));
                Config::default()
            }
        };

        App {
            state: State::new(config.mode),
            exit: false,
            config_tx: tx,
            toast,
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.handle_toast_action();
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(&self.state, area);
        self.draw_toast(frame);
    }

    /// Draw the list of toasts on top of everything
    fn draw_toast(&self, frame: &mut Frame) {
        let area = frame.area();
        let mut single_toast_area = Rect::new(0, 0, 30, 0);

        single_toast_area.x = area.width - single_toast_area.width;

        for message in &self.toast.messages {
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
            let line_count = paragraph.line_count(single_toast_area.width - 2);
            single_toast_area.height = line_count as u16;

            frame.render_widget(paragraph, single_toast_area);

            // update y for the next area
            single_toast_area.y += line_count as u16;
        }
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        if event::poll(Duration::from_millis(250))?
            && let Ok(event) = event::read()
        {
            if let Some(event::KeyEvent {
                code: KeyCode::Esc, ..
            }) = event.as_key_press_event()
            {
                self.exit = true
            }

            let transition = self.state.handle_events(event);
            self.handle_transition(transition);
        }

        let transition = self.state.on_tick();
        self.handle_transition(transition);

        Ok(())
    }

    fn handle_transition(&mut self, transition: Action) {
        match transition {
            Action::Quit => self.exit = true,
            Action::None => (),
            Action::UpdateMode(mode) => {
                let _ = self.config_tx.send(ConfigUpdate::Mode(mode));
            }
        }
    }

    fn handle_toast_action(&mut self) {
        if let Ok(action) = self.toast.action_rx.try_recv() {
            self.toast.handle_action(action);
        }
    }
}
