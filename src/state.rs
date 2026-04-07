use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Offset, Rect};
use ratatui::macros::{line, text};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::Line;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap};
use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::typing_test::TypingTest;
use crate::typing_test::mode_selection::ModeSelection;

pub enum Action {
    None,
    Quit,
    UpdateMode(Mode),
}

#[derive(Default)]
pub struct TypingStats {
    wpm: f64,
    current_index: usize,
    elapsed: Duration,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    Quote,
    Words(usize),
    Time(usize),
}

impl Mode {
    pub fn get_data(&self) -> Data {
        match self {
            Mode::Quote => Data::get_random_quote(),
            Mode::Words(n) => Data::get_n_random_words(*n),
            // TODO: new lines as the user reaches the end
            // max 80 char per line -> ~16 words
            // preload 4 lines
            //
            // NOTE: require refactor of current architecture or it will become messy
            // for now, just assume the user won't type more than 240 wpm
            Mode::Time(t) => {
                let mut data = Data::get_n_random_words(t * 4);
                data.source = format!("{} seconds", t);
                data
            }
        }
    }
}

pub struct State {
    // (time, wpm)
    history: Vec<(f64, f64)>,
    mode: Mode,
    data: Data,
    screen: Screen,
}

pub enum Screen {
    TypingTestState {
        typing_test: TypingTest,
        stats_last_updated_time: Instant,
        stats: TypingStats,
        selected_mode: ModeSelection,
    },
    EndScreenState {
        wpm: f64,
        accuracy: usize,
    },
}

impl Screen {
    /// Gets a fresh typing test
    pub fn new_typing_test(text: &str, mode: Mode) -> Self {
        Screen::TypingTestState {
            typing_test: TypingTest::new(text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
            selected_mode: ModeSelection::new(mode),
        }
    }

    /// Gets a new end screen
    pub fn new_end_screen(wpm: f64, accuracy: usize) -> Self {
        Screen::EndScreenState { wpm, accuracy }
    }
}

impl State {
    pub fn new(initial_mode: Mode) -> Self {
        let data = initial_mode.get_data();
        State {
            history: vec![],
            screen: Screen::new_typing_test(&data.text, initial_mode.clone()),
            mode: initial_mode,
            data,
        }
    }

    pub fn handle_events(&mut self, event: Event) -> Action {
        let screen = &mut self.screen;

        match screen {
            Screen::TypingTestState {
                typing_test,
                selected_mode,
                ..
            } => {
                if let Some(key) = event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char(c) => {
                            typing_test.start();

                            let has_ended = typing_test.on_type(c);
                            if has_ended {
                                let wpm = typing_test.net_wpm();
                                let accuracy = typing_test.accuracy();

                                if let Some(elapsed) = typing_test.elapsed_since_start_sec() {
                                    self.history.push((elapsed.as_secs_f64(), wpm));
                                }

                                self.screen = Screen::new_end_screen(wpm, accuracy);
                            }
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                        }
                        KeyCode::Tab => {
                            self.new_typing_test();
                        }
                        KeyCode::Left => {
                            selected_mode.handle_left();
                            let mode = selected_mode.selected_mode();
                            return self.update_mode_if_different(mode);
                        }
                        KeyCode::Right => {
                            selected_mode.handle_right();
                            let mode = selected_mode.selected_mode();
                            return self.update_mode_if_different(mode);
                        }
                        KeyCode::Up => {
                            selected_mode.handle_up();
                            let mode = selected_mode.selected_mode();
                            return self.update_mode_if_different(mode);
                        }
                        KeyCode::Down => {
                            selected_mode.handle_down();
                            let mode = selected_mode.selected_mode();
                            return self.update_mode_if_different(mode);
                        }
                        _ => {}
                    }
                }
            }
            Screen::EndScreenState { .. } => {
                if let Some(key) = event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Action::Quit;
                        }
                        KeyCode::Tab => {
                            self.new_typing_test();
                        }
                        _ => (),
                    };
                }
            }
        };

        Action::None
    }

    pub fn on_tick(&mut self) -> Action {
        match &mut self.screen {
            Screen::TypingTestState {
                typing_test,
                stats_last_updated_time,
                stats,
                ..
            } => {
                let elapsed = typing_test.elapsed_since_start_sec();
                if typing_test.has_started()
                    && matches!(elapsed, Some(duration) if duration > Duration::from_secs(1))
                {
                    let wpm = typing_test.net_wpm();

                    if stats_last_updated_time.elapsed() > Duration::from_secs(1) {
                        stats.wpm = wpm;
                        stats.current_index = typing_test.word_index;
                        stats.elapsed = elapsed.unwrap_or(Duration::from_secs(0));

                        *stats_last_updated_time = Instant::now();
                    }

                    if let Some(elapsed) = elapsed {
                        self.history.push((elapsed.as_secs_f64(), wpm));

                        if let Mode::Time(max_time) = self.mode
                            && elapsed > Duration::from_secs(max_time as u64)
                        {
                            let accuracy = typing_test.accuracy();
                            self.screen = Screen::new_end_screen(wpm, accuracy)
                        }
                    }
                }
            }
            Screen::EndScreenState { .. } => {}
        };

        Action::None
    }

    /// get a fresh typing test
    fn new_typing_test(&mut self) {
        self.history.clear();
        self.data = self.mode.get_data();
        self.screen = Screen::new_typing_test(&self.data.text, self.mode.clone());
    }

    /// Updates the typing test mode
    /// Also write to config file this is the last mode selected
    fn update_mode_if_different(&mut self, selected_mode: Option<Mode>) -> Action {
        if let Some(mode) = selected_mode
            && mode != self.mode
        {
            self.mode = mode.clone();
            self.new_typing_test();

            return Action::UpdateMode(mode.clone());
        }

        Action::None
    }

    /// Renders the menu of keybinds at the bottom
    fn render_bottom_menu_typing(area: Rect, buf: &mut Buffer) {
        let text = text![
            line!("Next <Tab>  Quit <Esc>"),
            line!("Select mode <Up/Down/Left/Right>"),
        ]
        .fg(Color::DarkGray)
        .centered();

        let mut menu_area = area.centered_horizontally(Constraint::Length(text.width() as u16));
        menu_area.y = area.bottom() - text.height() as u16;

        text.render(menu_area, buf);
    }

    /// Renders the menu of keybinds at the bottom
    fn render_bottom_menu_end_screen(area: Rect, buf: &mut Buffer) {
        let line = Line::raw("Next <Tab>  Quit <Esc/q>").fg(Color::DarkGray);
        let mut menu_area = area.centered_horizontally(Constraint::Length(line.width() as u16));
        menu_area.y = area.bottom() - 2;

        line.render(menu_area, buf);
    }

    /// Renders the wpm history graph
    /// If there are not data or the bounds are equal, ratatui's Chart handles it by showing no
    /// data.
    fn render_endscreen_graph(area: Rect, buf: &mut Buffer, history: &[(f64, f64)]) {
        let datasets = vec![
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .data(history),
        ];

        let max_wpm = history
            .iter()
            .map(|(_, wpm)| wpm.ceil() as i32)
            .max()
            .unwrap_or(0);

        // Make the graph go to 1 if it's less for prettier graph
        let max_wpm = if max_wpm <= 1 { 1.0 } else { max_wpm as f64 };
        let y_axis = Axis::default()
            .title("WPM")
            .style(Style::default().white())
            .bounds([0.0, max_wpm])
            .labels([
                "0.0".to_string(),
                format!("{:.1}", max_wpm / 2.0),
                format!("{:.1}", max_wpm),
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

impl Widget for &State {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = area.centered_horizontally(Constraint::Max(80));
        let typing_test_area = area.centered_vertically(Constraint::Length(3));

        match &self.screen {
            Screen::TypingTestState {
                typing_test,
                stats,
                selected_mode,
                ..
            } => {
                typing_test.render(typing_test_area, buf);

                let stats_area = typing_test_area.offset(Offset { x: 0, y: -2 });
                let wpm = stats.wpm;

                let line = match self.mode {
                    Mode::Time(t) => {
                        let elapsed = stats.elapsed;
                        let remaining = u64::max(0, t as u64 - elapsed.as_secs());
                        line![format!("{} {:.0}", remaining, wpm)]
                    }
                    _ => {
                        let cur_index = stats.current_index;
                        let n_words = typing_test.n_words();
                        line![format!("{}/{} {:.0}", cur_index, n_words, wpm)]
                    }
                };

                line.render(stats_area, buf);

                selected_mode.render(area, buf);
                State::render_bottom_menu_typing(area, buf);
            }
            Screen::EndScreenState { wpm, accuracy } => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);

                let text = text![
                    format!("WPM: {:.1}", wpm),
                    format!("ACC: {}%", accuracy),
                    format!(""),
                    format!("{}", self.data.source),
                ];
                let stats_area = layout[1].offset(Offset { x: 0, y: 2 });

                Paragraph::new(text)
                    .wrap(Wrap { trim: true })
                    .centered()
                    .render(stats_area, buf);

                State::render_endscreen_graph(layout[0], buf, &self.history);
                State::render_bottom_menu_end_screen(area, buf);
            }
        }
    }
}
