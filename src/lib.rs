use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use ratatui::DefaultTerminal;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::interval;

use self::action::Action;
use self::model::{AppModel, handle_action, update, view};
use self::msg::Msg;
use self::util::config::ConfigUpdate;
use self::util::toast::ToastAction;

pub mod action;
pub mod data;
mod endscreen;
mod model;
mod msg;
mod state;
mod typing_test;
mod util;

pub enum CustomEvent {
    Quit,
    Tick,
    Render,
    Key(KeyEvent),
    ToastAction(ToastAction),
    ConfigUpdate(ConfigUpdate),
}

pub async fn run(terminal: &mut DefaultTerminal, fps: usize, tps: usize) -> color_eyre::Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    init_event_loop(event_tx.clone(), fps, tps);

    let mut app_model = AppModel::init(event_tx).await;

    while !app_model.exit {
        let action: Option<Action> = tokio::select! {
            Some(custom_event) = event_rx.recv() => {
                match custom_event {
                    CustomEvent::Quit => Some(Action::Quit),
                    CustomEvent::Tick => update(&mut app_model, Msg::Tick),
                    CustomEvent::Render => {
                        terminal.draw(|frame| view(&app_model, frame))?;
                        None
                    }
                    CustomEvent::Key(key) => update(&mut app_model, Msg::Key(key)),
                    CustomEvent::ToastAction(action) => update(&mut app_model, Msg::ToastAction(action)),
                    CustomEvent::ConfigUpdate(config_update) => update(&mut app_model, Msg::ConfigUpdate(config_update)),

                }

            },
        };

        if let Some(action) = action {
            handle_action(&mut app_model, action);
        }
    }

    Ok(())
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
