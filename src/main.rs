use typing_test_tui::App;
use typing_test_tui::data::Data;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let data = Data::new_offline(None, None)?;
    let mut app = App::new(data);
    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
