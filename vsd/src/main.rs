mod automation;
mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod logger;
mod merger;
mod playlist;
mod progress;
mod utils;

use clap::{ColorChoice, Parser};
use colored::Colorize;
use commands::{Args, Commands};

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    kdam::term::init(match args.color {
        ColorChoice::Always => true,
        ColorChoice::Auto => std::io::IsTerminal::is_terminal(&std::io::stderr()),
        ColorChoice::Never => false,
    });

    match args.color {
        ColorChoice::Always => colored::control::set_override(true),
        ColorChoice::Never => colored::control::set_override(false),
        _ => (),
    }

    match args.command {
        #[cfg(feature = "browser")]
        Commands::Capture(args) => args.execute().await?,
        Commands::Extract(args) => args.execute()?,
        Commands::Merge(args) => args.execute()?,
        Commands::Save(args) => args.execute().await?,
    }

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    log::set_logger(&logger::Logger)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to initialize logger.");

    let mut symbols = requestty::symbols::UNICODE;
    symbols.completed = '•';
    symbols.cross = 'x';
    requestty::symbols::set(symbols);

    if let Err(e) = run().await {
        eprintln!("{}: {}", "error".bold().red(), e);
        std::process::exit(1);
    }
}
