use std::io::{self, Stdout};

use clap::Parser;
use crossterm::cursor;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use typing_test_tui::args::Args;
use typing_test_tui::run;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let res = {
        let mut term = setup_terminal()?;
        run(&mut term, args).await
    };

    if let Err(e) = teardown_terminal() {
        eprintln!("Error tearing down terminal: {}", e);
    }

    if let Err(ref e) = res {
        eprintln!("Error while running tui: {}", e);
    }

    res
}

fn setup_terminal() -> color_eyre::Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    Ok(terminal)
}

fn teardown_terminal() -> color_eyre::Result<()> {
    let mut stdout = io::stdout();
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}
