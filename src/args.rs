use clap::Parser;

// TODO: --offline mode uses my own data
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// How many times per second the tui is drawn.
    #[arg(short, long, default_value_t = 30)]
    pub fps: usize,

    /// The tick per second
    #[arg(short, long, default_value_t = 120)]
    pub tps: usize,

    /// Custom path to database of words. See
    /// https://github.com/mingli202/typing-test-tui/blob/main/assets/english.json for shape of
    /// json
    #[arg(short, long)]
    pub words_path: Option<String>,

    /// Custom path to database of quotes. See
    /// https://github.com/mingli202/typing-test-tui/blob/main/assets/quotes.json for shape of json
    #[arg(short, long)]
    pub quotes_path: Option<String>,
}
