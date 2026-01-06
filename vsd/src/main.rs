mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod merger;
mod playlist;
mod utils;
mod automation;

use clap::{ColorChoice, Parser};
use commands::{Args, Commands};
use kdam::{term, term::Colorizer};
use requestty::symbols;
use std::{
    io::{stderr, IsTerminal},
    process,
};

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    term::init(match args.color {
        ColorChoice::Always => true,
        ColorChoice::Auto => stderr().is_terminal(),
        ColorChoice::Never => false,
    });

    match args.command {
        #[cfg(feature = "browser")]
        Commands::Capture(args) => args.execute()?,
        Commands::Extract(args) => args.execute()?,
        Commands::Merge(args) => args.execute()?,
        Commands::Save(args) => args.execute().await?,
    }

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut symbols = symbols::UNICODE;
    symbols.completed = '•';
    symbols.cross = 'x';
    symbols::set(symbols);

    if let Err(e) = run().await {
        eprintln!("{}: {}", "error".colorize("bold red"), e);
        process::exit(1);
    }
}
