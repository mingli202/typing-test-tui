use std::io;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui::{DefaultTerminal, Frame};

mod typing_test;

#[derive(Default)]
pub struct TypingTestState {}

#[derive(Default)]
pub struct EndScreenState {}

pub enum Transition {
    None,
    Switch(State),
    Push(State),
    Pop,
    Quit,
}

pub enum State {
    TypingTestState(TypingTestState),
    EndScreenState(EndScreenState),
}

impl Default for State {
    fn default() -> Self {
        State::TypingTestState(TypingTestState::default())
    }
}

impl Widget for &State {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        todo!()
    }
}

impl State {
    fn handle_events(&mut self, event: Event) -> Transition {
        Transition::None
    }
}

#[derive(Default)]
struct Config {}

#[derive(Default)]
struct App {
    state: State,
    history: Vec<State>,
    config: Config,
    exit: bool,
}

impl App {
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
