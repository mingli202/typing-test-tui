use std::io;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
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
    EndScreenState,
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
            State::EndScreenState => {}
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
        Transition::None
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
            let transition = self.state.handle_events(event);
            self.handle_transition(transition);
        }
        Ok(())
    }

    fn handle_transition(&mut self, transition: Transition) {
        match transition {
            Transition::Switch(next_state) => self.state = next_state,
            Transition::Quit => self.exit = true,
            Transition::Push(state) => self.history.push(state),
            Transition::Pop => {
                self.history.pop();
            }
            Transition::None => (),
        }
    }
}
