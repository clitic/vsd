mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod merger;
mod playlist;
mod update;
mod utils;

use clap::{ColorChoice, Parser};
use commands::{Args, Commands};
use kdam::{term, term::Colorizer};
use requestty::symbols;
use std::{
    io::{stderr, IsTerminal},
    process,
};

fn run() -> anyhow::Result<()> {
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
        Commands::Save(args) => args.execute()?,
    }

    Ok(())
}

fn main() {
    let mut symbols = symbols::UNICODE;
    symbols.completed = '•';
    symbols.cross = 'x';
    symbols::set(symbols);

    if let Err(e) = run() {
        eprintln!("{}: {}", "error".colorize("bold red"), e);
        process::exit(1);
    }
}

/*
    TODOs

    1. Add resume support
    2. Create a custom thread pool module
    3. Reduce dependency on anyhow crate
    4. Reduce dependency on ffmpeg
*/
