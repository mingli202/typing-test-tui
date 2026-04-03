use std::time::{Duration, Instant};

use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Offset, Rect};
use ratatui::macros::{line, span, text};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap};

use crate::data::Data;
use crate::typing_test::TypingTest;

pub enum Action {
    None,
    Switch(State),
    Quit,
}

#[derive(Default)]
pub struct TypingStats {
    wpm: f64,
    current_index: usize,
}

#[derive(Clone)]
pub enum Mode {
    Quote,
    Words(usize),
}

impl Mode {
    pub fn get_data(&self) -> Data {
        match self {
            Mode::Quote => Data::get_random_quote(),
            Mode::Words(n) => Data::get_n_random_words(*n),
        }
    }
}

pub struct State {
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
    },
    EndScreenState {
        wpm: f64,
        accuracy: usize,
    },
}

impl Screen {
    /// Gets a fresh typing test
    pub fn new_typing_test(text: &str) -> Self {
        Screen::TypingTestState {
            typing_test: TypingTest::new(text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
        }
    }

    /// Gets a new end screen
    pub fn new_end_screen(wpm: f64, accuracy: usize) -> Self {
        Screen::EndScreenState { wpm, accuracy }
    }
}

impl State {
    pub fn new() -> Self {
        let mode = Mode::Words(10);
        let data = mode.get_data();
        State {
            history: vec![],
            screen: Screen::new_typing_test(&data.text),
            mode,
            data,
        }
    }

    pub fn handle_events(&mut self, event: Event) -> Action {
        match &mut self.screen {
            Screen::TypingTestState { typing_test, .. } => {
                if let Some(key) = event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char(c) => {
                            typing_test.start();

                            let has_ended = typing_test.on_type(c);
                            if has_ended {
                                let wpm = typing_test.net_wpm();
                                let accuracy = typing_test.accuracy();
                                self.screen = Screen::new_end_screen(wpm, accuracy);
                            }
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                        }
                        KeyCode::Tab => {
                            self.new_typing_test();
                        }
                        KeyCode::Left => {}
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
                    let wpm = typing_test.current_net_wpm();

                    if stats_last_updated_time.elapsed() > Duration::from_secs(1) {
                        stats.wpm = wpm;
                        stats.current_index = typing_test.word_index;

                        *stats_last_updated_time = Instant::now();
                    }

                    if let Some(elapsed) = elapsed {
                        self.history.push((elapsed.as_secs_f64(), wpm));
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
        self.screen = Screen::new_typing_test(&self.data.text);
    }

    /// Renders the menu of keybinds at the bottom
    fn render_bottom_menu_typing(area: Rect, buf: &mut Buffer) {
        let text = text![
            line!("Next <Tab>  Quit <Esc>"),
            line!("Select mode <Up/Down/Left/Right>"),
        ]
        .fg(Color::Gray)
        .centered();

        let mut menu_area = area.centered_horizontally(Constraint::Length(text.width() as u16));
        menu_area.y = area.bottom() - text.height() as u16;

        text.render(menu_area, buf);
    }

    /// Renders the menu of keybinds at the bottom
    fn render_bottom_menu_end_screen(area: Rect, buf: &mut Buffer) {
        let line = Line::raw("Next <Tab>  Quit <Esc/q>").fg(Color::Gray);
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

    /// Render mode selection at the top
    fn render_mode_selection(area: Rect, buf: &mut Buffer, mode: &Mode) {
        fn highlight(text: Span) -> Span {
            text.fg(Color::Black).bg(Color::White)
        }

        let mut quote_text = span!("Quote");
        let mut word_text = span!("Words");

        let mut choices = vec![span!("10"), span!("25"), span!("50"), span!("100")];

        match mode {
            Mode::Quote => {
                quote_text = highlight(quote_text);
            }
            Mode::Words(n) => {
                let n = n.to_string();
                word_text = highlight(word_text);

                if let Some(chosen) = choices.iter_mut().find(|choice| *choice.content == n) {
                    *chosen = highlight(chosen.clone());
                }
            }
        }

        let choices: Vec<Span> =
            itertools::Itertools::intersperse(choices.into_iter(), span!(" ")).collect();

        let selection = text![
            line![quote_text, span!(" "), word_text],
            span!(" "),
            Line::from(choices)
        ];

        selection.centered().render(area, buf);
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
                typing_test, stats, ..
            } => {
                typing_test.render(typing_test_area, buf);

                let wpm = stats.wpm;
                let cur_index = stats.current_index;
                let n_words = typing_test.n_words();
                let stats_area = typing_test_area.offset(Offset { x: 0, y: -2 });
                let line = line![format!("{}/{} {:.0}", cur_index, n_words, wpm)];

                line.render(stats_area, buf);

                State::render_mode_selection(area, buf, &self.mode);
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
