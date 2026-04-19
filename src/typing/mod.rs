use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Offset, Rect};
use ratatui::macros::{line, text};
use ratatui::style::{Color, Stylize};
use ratatui::widgets::Widget;

use crate::action::Action;
use crate::endscreen::EndScreenModel;
use crate::model::{Mode, Screen, SharedModel};
use crate::util::data_provider::DataProvider;

use self::mode_selection::ModeSelection;
use self::typing::TypingTest;

mod letter;
mod mode_selection;
mod typing;
mod word;

pub enum Msg {
    Tick,
    Key(KeyCode),
    UpdateMode(Mode),
}

#[derive(Debug, Default)]
pub struct TypingStats {
    wpm: f64,
    current_index: usize,
    elapsed: Duration,
}

pub struct TypingModel {
    typing_test: TypingTest,
    stats_last_updated_time: Instant,
    stats: TypingStats,
    selected_mode: ModeSelection,
}

impl TypingModel {
    pub fn new(text: &str, initial_mode: Mode) -> Self {
        TypingModel {
            typing_test: TypingTest::new(text),
            stats_last_updated_time: Instant::now(),
            stats: TypingStats::default(),
            selected_mode: ModeSelection::new(initial_mode),
        }
    }
}

pub fn update(
    typing_model: &mut TypingModel,
    shared_model: &mut SharedModel,
    data_provider: &DataProvider,
    msg: Msg,
) -> Option<Action> {
    let TypingModel {
        typing_test,
        stats_last_updated_time,
        stats,
        selected_mode,
    } = typing_model;

    match msg {
        Msg::Key(key) => match key {
            KeyCode::Char(c) => {
                typing_test.start();

                let has_ended = typing_test.on_type(c);
                if has_ended {
                    let wpm = typing_test.net_wpm();
                    let accuracy = typing_test.accuracy();

                    if let Some(elapsed) = typing_test.elapsed_since_start_sec() {
                        shared_model.history.push((elapsed.as_secs_f64(), wpm));
                    }

                    return Some(Action::SwitchScreen(Screen::End(EndScreenModel::new(
                        wpm, accuracy,
                    ))));
                }
            }
            KeyCode::Backspace => {
                typing_test.on_backspace();
            }
            KeyCode::Tab => {
                return Some(Action::new_typing_screen(shared_model, data_provider));
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                return handle_arrow_keys(selected_mode, shared_model, key);
            }
            _ => {}
        },
        Msg::Tick => {
            let elapsed = typing_test.elapsed_since_start_sec();
            if typing_test.has_started()
                && let Some(elapsed) = elapsed
                && elapsed > Duration::from_secs(1)
            {
                if let Mode::Time(max_time) = shared_model.mode
                    && elapsed > Duration::from_secs(max_time as u64)
                {
                    let accuracy = typing_test.accuracy();
                    let wpm = typing_test.net_wpm();

                    shared_model.history.push((elapsed.as_secs_f64(), wpm));

                    return Some(Action::new_end_screen(wpm, accuracy));
                }

                if stats_last_updated_time.elapsed() > Duration::from_secs(1) {
                    *stats_last_updated_time = Instant::now();

                    let wpm = typing_test.net_wpm();

                    stats.wpm = wpm;
                    stats.current_index = typing_test.word_index;

                    shared_model.history.push((elapsed.as_secs_f64(), wpm));
                }

                stats.elapsed = elapsed
            }
        }
        Msg::UpdateMode(new_mode) => {
            shared_model.mode = new_mode.clone();
            return Some(Action::new_typing_screen(shared_model, data_provider));
        }
    };

    None
}

/// Arrow keys change the current mode
fn handle_arrow_keys(
    selected_mode: &mut ModeSelection,
    shared_model: &SharedModel,
    key: KeyCode,
) -> Option<Action> {
    match key {
        KeyCode::Left => {
            selected_mode.handle_left();
        }
        KeyCode::Right => {
            selected_mode.handle_right();
        }
        KeyCode::Up => {
            selected_mode.handle_up();
        }
        KeyCode::Down => {
            selected_mode.handle_down();
        }
        _ => {}
    }
    let selected_mode = selected_mode.selected_mode();

    if let Some(selected_mode) = selected_mode
        && selected_mode != shared_model.mode
    {
        return Some(Action::ModeChange(selected_mode));
    }

    None
}

/// Main view function for typing test screen
pub fn view(typing_model: &TypingModel, shared_model: &SharedModel, area: Rect, buf: &mut Buffer) {
    let typing_test_area = area.centered_vertically(Constraint::Length(3));
    typing::view_typing_test(&typing_model.typing_test, typing_test_area, buf);

    view_stats(
        &typing_model.stats,
        &shared_model.mode,
        typing_model.typing_test.n_words(),
        typing_test_area,
        buf,
    );

    mode_selection::view_mode_selection(&typing_model.selected_mode, area, buf);
    view_bottom_menu_typing(area, buf);
}

/// Render stats
fn view_stats(
    stats: &TypingStats,
    mode: &Mode,
    n_words: usize,
    typing_test_area: Rect,
    buf: &mut Buffer,
) {
    let stats_area = typing_test_area.offset(Offset { x: 0, y: -2 });
    let wpm = stats.wpm;

    let line = match mode {
        Mode::Time(t) => {
            let elapsed = stats.elapsed;
            let remaining = u64::max(0, (*t as u64).saturating_sub(elapsed.as_secs()));
            line![format!("{} {:.0}", remaining, wpm)]
        }
        _ => {
            let cur_index = stats.current_index;
            line![format!("{}/{} {:.0}", cur_index, n_words, wpm)]
        }
    };

    line.render(stats_area, buf);
}

/// Render some instructions
fn view_bottom_menu_typing(area: Rect, buf: &mut Buffer) {
    let text = text![
        line!("Next <Tab>  Quit <Esc>"),
        line!("Select mode <Up/Down/Left/Right>"),
    ]
    .fg(Color::DarkGray)
    .centered();

    let mut menu_area = area.centered_horizontally(Constraint::Length(text.width() as u16));
    menu_area.y = area.bottom().saturating_sub(text.height() as u16);

    text.render(menu_area, buf);
}
