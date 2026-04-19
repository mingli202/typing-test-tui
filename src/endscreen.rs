use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Offset, Rect};
use ratatui::macros::text;
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::Line;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap};

use crate::action::Action;
use crate::model::SharedModel;
use crate::util::data_provider::DataProvider;

pub enum Msg {
    Key(KeyCode),
}

pub struct EndScreenModel {
    final_wpm: f64,
    accuracy: usize,
}

impl EndScreenModel {
    pub fn new(final_wpm: f64, accuracy: usize) -> Self {
        EndScreenModel {
            final_wpm,
            accuracy,
        }
    }
}

pub fn update(
    shared_model: &mut SharedModel,
    data_provider: &DataProvider,
    msg: Msg,
) -> Option<Action> {
    let Msg::Key(key) = msg;
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            return Some(Action::Quit);
        }
        KeyCode::Tab => return Some(Action::new_typing_screen(shared_model, data_provider)),
        _ => (),
    }

    None
}

pub fn view(model: &EndScreenModel, shared_model: &SharedModel, area: Rect, buf: &mut Buffer) {
    let EndScreenModel {
        final_wpm,
        accuracy,
    } = model;

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let text = text![
        format!("WPM: {:.1}", final_wpm),
        format!("ACC: {}%", accuracy),
        format!(""),
        format!("{}", shared_model.data.source),
    ];
    let stats_area = layout[1].offset(Offset { x: 0, y: 2 });

    Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .centered()
        .render(stats_area, buf);

    render_endscreen_graph(layout[0], buf, &shared_model.history);
    render_bottom_menu_end_screen(area, buf);
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
