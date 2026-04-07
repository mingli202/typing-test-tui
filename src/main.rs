use tokio::sync::mpsc;
use typing_test_tui::App;
use typing_test_tui::config::Config;
use typing_test_tui::toast::Toast;

use clap::Parser;

// TODO: --offline mode uses my own data
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let _ = Args::parse();

    let (config_tx, config_rx) = mpsc::unbounded_channel();
    let (toast_tx, toast_rx) = mpsc::unbounded_channel();

    let toast = Toast::new(toast_tx.clone());
    let toast_handle = toast.init(toast_rx);

    let config_handle = Config::init(config_rx, toast_tx);

    {
        let app = App::new(config_tx, toast).await;
        ratatui::run(|terminal| app.run(terminal))?;
    }

    toast_handle.await?;
    config_handle.await?;

    Ok(())
}
