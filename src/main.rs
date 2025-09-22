use clap::Parser;
use retrochat::cli::Cli;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    cli.run()
}
