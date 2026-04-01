use std::io;
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Offset, Rect};
use ratatui::macros::{line, text};
use ratatui::style::{Color, Stylize};
use ratatui::text::Line;
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

#[derive(Default)]
pub struct TypingStats {
    wpm: f32,
    current_index: usize,
}

pub enum State {
    TypingTestState {
        typing_test: TypingTest,
        stats_last_updated_time: Instant,
        stats: TypingStats,
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
        let typing_test_area = area
            .centered_vertically(Constraint::Length(3))
            .centered_horizontally(Constraint::Max(80));

        match self {
            State::TypingTestState {
                typing_test, stats, ..
            } => {
                typing_test.render(typing_test_area, buf);

                let wpm = stats.wpm;
                let cur_index = stats.current_index;
                let n_words = typing_test.n_words();
                let stats_area = typing_test_area.offset(Offset { x: 0, y: -2 });
                let line = line![format!("{}/{} {:.0}", cur_index, n_words, wpm)];

                line.render(stats_area, buf);
            }
            State::EndScreenState {
                wpm,
                accuracy,
                source,
            } => {
                let text = text![format!("WPM: {:.1}", wpm), format!("ACC: {}%", accuracy),];
                let area = area.centered(
                    Constraint::Length(text.width() as u16),
                    Constraint::Length(text.height() as u16),
                );

                Paragraph::new(text).centered().render(area, buf);
            }
        }

        State::render_bottom_menu(area, buf);
    }
}

impl State {
    pub fn new(initial_text: &str) -> Self {
        State::TypingTestState {
            typing_test: TypingTest::new(initial_text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
        }
    }

    fn handle_events(app: &mut App, event: Event) -> Transition {
        match &mut app.state {
            State::TypingTestState { typing_test, .. } => {
                if let Some(key) = event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char(c) => {
                            typing_test.start();

                            if typing_test.on_type(c) {
                                let wpm = typing_test.net_wpm();
                                let accuracy = typing_test.accuracy();
                                return Transition::Switch(State::EndScreenState {
                                    wpm,
                                    accuracy,
                                    source: "".to_string(),
                                });
                            }

                            Transition::None
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                            Transition::None
                        }
                        KeyCode::Tab => Transition::Switch(State::TypingTestState {
                            typing_test: TypingTest::new(&app.data.get_random_quote().quote),
                            stats_last_updated_time: Instant::now(),
                            stats: TypingStats::default(),
                        }),
                        _ => Transition::None,
                    }
                } else {
                    Transition::None
                }
            }
            State::EndScreenState { .. } => {
                if let Some(key) = event.as_key_press_event() {
                    return match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Transition::Quit,
                        KeyCode::Tab => Transition::Switch(State::TypingTestState {
                            typing_test: TypingTest::new(&app.data.get_random_quote().quote),
                            stats_last_updated_time: Instant::now(),
                            stats: TypingStats::default(),
                        }),
                        _ => Transition::None,
                    };
                }

                Transition::None
            }
        }
    }

    fn on_tick(app: &mut App) {
        match &mut app.state {
            Self::TypingTestState {
                typing_test,
                stats_last_updated_time,
                stats,
            } => {
                if typing_test.has_started()
                    && stats_last_updated_time.elapsed() > Duration::from_secs(1)
                {
                    stats.wpm = typing_test.current_net_wpm();
                    stats.current_index = typing_test.word_index;
                    *stats_last_updated_time = Instant::now();
                }
            }
            Self::EndScreenState { .. } => {}
        }
    }

    /// Renders the menu of keybinds at the bottom
    fn render_bottom_menu(area: Rect, buf: &mut Buffer) {
        let line = Line::raw("Next <Tab>  Quit <Esc>").fg(Color::Gray);
        let mut menu_area = area.centered_horizontally(Constraint::Length(line.width() as u16));
        menu_area.y = area.bottom() - 2;

        line.render(menu_area, buf);
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
            self.handle_events()?;
            State::on_tick(self);
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(&self.state, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(250))?
            && let Ok(event) = event::read()
        {
            if let Some(event::KeyEvent {
                code: KeyCode::Esc, ..
            }) = event.as_key_press_event()
            {
                self.exit = true
            }

            let transition = State::handle_events(self, event);
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
