use tokio::sync::mpsc;
use typing_test_tui::App;
use typing_test_tui::config::Config;
use typing_test_tui::toast::Toast;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (config_tx, config_rx) = mpsc::unbounded_channel();
    let (toast_tx, toast_rx) = mpsc::unbounded_channel();

    let toast = Toast::new(toast_tx.clone());
    let toast_handle = toast.init(toast_rx);

    let handle = Config::init(config_rx, toast_tx);

    {
        let app = App::new(config_tx, toast).await;
        ratatui::run(|terminal| app.run(terminal))?;
    }

    toast_handle.await?;
    handle.await?;

    Ok(())
}
