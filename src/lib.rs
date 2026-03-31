use std::io;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::Rect;
use ratatui::macros::text;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

use self::data::Data;
use self::typing_test::TypingTest;

pub mod data;
mod typing_test;

pub enum Transition {
    None,
    Switch(State),
    Push(State),
    Pop,
    Quit,
}

pub enum State {
    TypingTestState {
        typing_test: TypingTest,
        is_typing: bool,
    },
    EndScreenState {
        wpm: f32,
        accuracy: usize,
        source: String,
    },
}

impl Widget for &State {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            State::TypingTestState {
                typing_test,
                is_typing,
            } => {
                typing_test.render(area, buf);
            }
            State::EndScreenState {
                wpm,
                accuracy,
                source,
            } => {
                Paragraph::new(text![
                    format!("WPM: {:.1}", wpm),
                    format!("ACC: {}%", accuracy),
                ])
                .centered()
                .render(area, buf);
            }
        }
    }
}

impl State {
    pub fn new(initial_text: &str) -> Self {
        State::TypingTestState {
            typing_test: TypingTest::new(initial_text),
            is_typing: false,
        }
    }

    fn handle_events(&mut self, event: Event) -> Transition {
        match self {
            State::TypingTestState {
                typing_test,
                is_typing,
            } => {
                if let Some(key) = event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char(c) => {
                            if typing_test.on_type(c) {
                                let wpm = typing_test.net_wpm();
                                let accuracy = typing_test.accuracy();
                                Transition::Switch(State::EndScreenState {
                                    wpm,
                                    accuracy,
                                    source: "".to_string(),
                                })
                            } else {
                                Transition::None
                            }
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                            Transition::None
                        }
                        KeyCode::Tab => {
                            typing_test.reset();
                            Transition::None
                        }
                        _ => Transition::None,
                    }
                } else {
                    Transition::None
                }
            }
            _ => Transition::None,
        }
    }
}

pub struct App {
    state: State,
    history: Vec<State>,
    exit: bool,
    data: Data,
}

impl App {
    pub fn new(data: Data) -> Self {
        let initial_text = data.get_random_quote().quote.clone();
        App {
            state: State::new(&initial_text),
            history: vec![],
            exit: false,
            data,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(&self.state, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Ok(event) = event::read() {
            if let Some(event::KeyEvent {
                code: KeyCode::Esc, ..
            }) = event.as_key_press_event()
            {
                self.exit = true
            }

            let transition = self.state.handle_events(event);
            self.handle_transition(transition);
        }
        Ok(())
    }

    fn handle_transition(&mut self, transition: Transition) {
        match transition {
            Transition::Switch(next_state) => self.state = next_state,
            Transition::Quit => self.exit = true,
            Transition::Push(state) => {
                self.history.push(state);
            }
            Transition::Pop => {
                self.history.pop();
            }
            Transition::None => (),
        }
    }
}
