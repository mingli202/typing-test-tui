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
}

pub async fn run(terminal: &mut DefaultTerminal, fps: usize, tps: usize) -> color_eyre::Result<()> {
    let mut app_model = AppModel::init(model::Mode::Quote);

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    init_event_loop(event_tx, fps, tps);

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
                }

            }
        };

        if let Some(action) = action {
            handle_action(&mut app_model, action);
        }
    }

    Ok(())
}

// fn draw(&self, frame: &mut Frame) {
//     let area = frame.area();
//     // frame.render_widget(&self.state, area);
//     self.draw_toast(frame);
// }
//
// /// Draw the list of toasts on top of everything
// fn draw_toast(&self, frame: &mut Frame) {
//     let area = frame.area();
//     let mut single_toast_area = Rect::new(0, 0, 30, 0);
//
//     single_toast_area.x = area.width - single_toast_area.width;
//
//     for message in &self.toast.messages {
//         let paragraph =
//             Paragraph::new(text![message.msg.clone()].fg(Color::White).bg(Color::Black))
//                 .black()
//                 .wrap(Wrap { trim: true })
//                 .block(
//                     Block::bordered()
//                         .border_style(message.level.style())
//                         .border_type(BorderType::Rounded),
//                 );
//
//         // calculate height after wrap
//         // -2 because it seems it doesn't handle the border
//         let line_count = paragraph.line_count(single_toast_area.width - 2);
//         single_toast_area.height = line_count as u16;
//
//         frame.render_widget(paragraph, single_toast_area);
//
//         // update y for the next area
//         single_toast_area.y += line_count as u16;
//     }
// }

// fn handle_key(&mut self, key: KeyEvent) -> Action {
//     if let KeyEvent {
//         code: KeyCode::Esc, ..
//     }
//     | KeyEvent {
//         code: KeyCode::Char('c'),
//         modifiers: KeyModifiers::CONTROL,
//         ..
//     } = key
//     {
//         self.exit = true
//     }
//
//     Action::Message(Msg::Key(key))
// }
//
// fn handle_actions(&mut self, actions: Vec<Action>) {
//     for action in actions {
//         match action {
//             Action::Quit => self.exit = true,
//             Action::UpdateConfigMode(mode) => {
//                 let _ = self.config_tx.send(ConfigUpdate::Mode(mode));
//             }
//             Action::Message(msg) => {
//                 let actions = update(&mut self.model, msg);
//                 self.handle_actions(actions);
//             }
//         }
//     }
// }

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
