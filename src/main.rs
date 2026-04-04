use tokio::sync::mpsc;
use typing_test_tui::App;
use typing_test_tui::config::Config;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (tx, rx) = mpsc::unbounded_channel();

    Config::init(rx).await;
    let mut app = App::new(tx).await;
    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
