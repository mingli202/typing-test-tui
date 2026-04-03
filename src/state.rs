use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Offset, Rect};
use ratatui::macros::{line, text};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::Line;
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

    pub fn handle_events(&mut self, event: Event) -> Action {
        match self {
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

                            let has_ended = typing_test.on_type(c);
                            if has_ended {
                                let wpm = typing_test.net_wpm();
                                let accuracy = typing_test.accuracy();
                                return Action::Switch(State::EndScreenState {
                                    wpm,
                                    accuracy,
                                    source: data.source.clone(),
                                    history: history.clone(),
                                });
                            }

                            Action::None
                        }
                        KeyCode::Backspace => {
                            typing_test.on_backspace();
                            Action::None
                        }
                        KeyCode::Tab => Action::Switch(State::new_typing_test()),
                        _ => Action::None,
                    }
                } else {
                    Action::None
                }
            }
            State::EndScreenState { .. } => {
                if let Some(key) = event.as_key_press_event() {
                    return match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                        KeyCode::Tab => Action::Switch(State::new_typing_test()),
                        _ => Action::None,
                    };
                }

                Action::None
            }
        }
    }

    pub fn on_tick(&mut self) -> Action {
        match self {
            Self::TypingTestState {
                typing_test,
                stats_last_updated_time,
                stats,
                history,
                ..
            } => {
                if typing_test.has_started()
                    && matches!(typing_test.elapsed_since_start_sec(), Some(duration) if duration > Duration::from_secs(1))
                {
                    let wpm = typing_test.current_net_wpm();
                    let wpm = if wpm < 0.0 { 0.0 } else { wpm };

                    if stats_last_updated_time.elapsed() > Duration::from_secs(1) {
                        stats.wpm = wpm;
                        stats.current_index = typing_test.word_index;

                        *stats_last_updated_time = Instant::now();
                    }

                    if let Some(elapsed) = typing_test.elapsed_since_start_sec() {
                        history.push((elapsed.as_secs_f64(), wpm));
                    }
                }
            }
            Self::EndScreenState { .. } => {}
        };

        Action::None
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
