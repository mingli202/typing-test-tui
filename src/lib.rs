use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, MouseEvent};
use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::macros::text;
use ratatui::style::{Color, Stylize};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::interval;

use self::config::{Config, ConfigUpdate};
use self::state::{Action, State};
use self::toast::{Toast, ToastMessage};

pub mod config;
pub mod data;
mod selection;
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
                let _ = toast.send(ToastMessage::warning(e));
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

    pub async fn run(
        mut self,
        terminal: &mut DefaultTerminal,
        fps: usize,
        tps: usize,
    ) -> color_eyre::Result<()> {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        init_event_loop(event_tx, fps, tps);

        while !self.exit
            && let Some(custom_event) = event_rx.recv().await
        {
            match custom_event {
                CustomEvent::Quit => self.exit = false,
                CustomEvent::Tick => {
                    let transition = self.state.on_tick();
                    self.handle_transition(transition);
                }
                CustomEvent::Render => {
                    terminal.draw(|frame| self.draw(frame))?;
                }
                CustomEvent::Key(key) => self.handle_key(key)?,
                CustomEvent::Mouse(_) => {}
            }
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
            // -2 because it seems it doesn't handle the border
            let line_count = paragraph.line_count(single_toast_area.width - 2);
            single_toast_area.height = line_count as u16;

            frame.render_widget(paragraph, single_toast_area);

            // update y for the next area
            single_toast_area.y += line_count as u16;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        if let KeyEvent {
            code: KeyCode::Esc, ..
        }
        | KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } = key
        {
            self.exit = true
        }

        let transition = self.state.handle_key(key);
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
        while let Ok(action) = self.toast.action_rx.try_recv() {
            self.toast.handle_action(action);
        }
    }
}

pub enum CustomEvent {
    Quit,
    Tick,
    Render,
    Key(KeyEvent),
    Mouse(MouseEvent),
}

fn init_event_loop(event_tx: UnboundedSender<CustomEvent>, fps: usize, tps: usize) {
    tokio::spawn(async move {
        let render_duration_secs = 1.0 / fps as f64;
        let tick_duration_secs = 1.0 / tps as f64;

        let mut tick_interval = interval(Duration::from_secs_f64(tick_duration_secs));
        let mut render_interval = interval(Duration::from_secs_f64(render_duration_secs));

        let mut event_stream = EventStream::new();

        loop {
            select! {
                _ = tick_interval.tick() => {
                    let _ = event_tx.send(CustomEvent::Tick);
                }
                _ = render_interval.tick() => {
                    let _ = event_tx.send(CustomEvent::Render);
                }
                maybe_event = event_stream.next().fuse() => {
                    let custom_event = match maybe_event {
                        Some(Ok(e)) => {
                            match e {
                                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => CustomEvent::Key(key_event),
                                Event::Mouse(mouse_event) => CustomEvent::Mouse(mouse_event),
                                _ => continue,
                            }
                        }
                        Some(Err(_)) => continue,
                        None => break,
                    };

                    if event_tx.send(custom_event).is_err() {
                        break;
                    }
                }
            }
        }
    });
}
