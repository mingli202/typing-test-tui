use std::io;
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Offset, Rect};
use ratatui::macros::{line, text};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::Line;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap};
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
    wpm: f64,
    current_index: usize,
}

pub enum State {
    TypingTestState {
        typing_test: TypingTest,
        stats_last_updated_time: Instant,
        stats: TypingStats,
        data: Data,
        history: Vec<(f64, f64)>,
    },
    EndScreenState {
        wpm: f64,
        accuracy: usize,
        source: String,
        history: Vec<(f64, f64)>,
    },
}

impl Widget for &State {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = area.centered_horizontally(Constraint::Max(80));
        let typing_test_area = area.centered_vertically(Constraint::Length(3));

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
                history,
                ..
            } => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

                let text = text![
                    format!("WPM: {:.1}", wpm),
                    format!("ACC: {}%", accuracy),
                    format!(""),
                    format!("{}", source),
                ];
                let stats_area = layout[1].offset(Offset { x: 0, y: 2 });

                Paragraph::new(text)
                    .wrap(Wrap { trim: true })
                    .centered()
                    .render(stats_area, buf);

                State::render_endscreen_graph(layout[0], buf, history);
            }
        }

        State::render_bottom_menu(area, buf);
    }
}

impl State {
    pub fn new_typing_test() -> Self {
        let data = Data::get_random_quote();

        State::TypingTestState {
            typing_test: TypingTest::new(&data.text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
            data,
            history: vec![],
        }
    }

    fn handle_events(app: &mut App, event: Event) -> Transition {
        match &mut app.state {
            State::TypingTestState {
                typing_test,
                data,
                history,
                ..
            } => {
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
                                    source: data.source.clone(),
                                    history: history.clone(),
                                });
                            }

                            Transition::None
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                            Transition::None
                        }
                        KeyCode::Tab => Transition::Switch(State::new_typing_test()),
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
                        KeyCode::Tab => Transition::Switch(State::new_typing_test()),
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
                history,
                ..
            } => {
                if typing_test.has_started()
                    && matches!(typing_test.elapsed_since_start_sec(), Some(duration) if duration > Duration::from_secs(1))
                    && stats_last_updated_time.elapsed() > Duration::from_secs(1)
                {
                    stats.wpm = typing_test.current_net_wpm();
                    stats.current_index = typing_test.word_index;

                    if let Some(elapsed) = typing_test.elapsed_since_start_sec() {
                        history.push((elapsed.as_secs_f64(), stats.wpm));
                    }

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

    fn render_endscreen_graph(area: Rect, buf: &mut Buffer, history: &[(f64, f64)]) {
        let datasets = vec![
            Dataset::default()
                .name("wpm history")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .data(history),
        ];

        let max_wpm = history
            .iter()
            .map(|(_, wpm)| wpm.ceil() as i32)
            .max()
            .unwrap_or(0);

        let y_axis = Axis::default()
            .title("wpm")
            .style(Style::default().white())
            .bounds([0.0, max_wpm as f64])
            .labels([
                "0".to_string(),
                (max_wpm / 2).to_string(),
                max_wpm.to_string(),
            ]);

        let first_instant = history.first().unwrap_or(&(0.0, 0.0)).0;
        let last_instant = history.last().unwrap_or(&(0.0, 0.0)).0;

        let x_axis = Axis::default()
            .title("time (s)")
            .style(Style::default().white())
            .bounds([first_instant, last_instant])
            .labels([
                format!("{:.0}", first_instant),
                format!("{:.0}", last_instant / 2.0),
                format!("{:.1}", last_instant),
            ]);

        Chart::new(datasets)
            .x_axis(x_axis)
            .y_axis(y_axis)
            .render(area, buf);
    }
}

pub struct App {
    state: State,
    history: Vec<State>,
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            state: State::new_typing_test(),
            history: vec![],
            exit: false,
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

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
