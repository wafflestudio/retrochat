use clap::Parser;
use retrochat::cli::Cli;

fn main() -> anyhow::Result<()> {
    // Load .env file if it exists (ignore errors if missing)
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    cli.run()
}
