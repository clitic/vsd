mod commands;
mod cookie;
mod dash;
mod downloader;
mod error;
mod hls;
mod merger;
mod options;
mod playlist;
mod update;
mod utils;

use clap::{ColorChoice, Parser};
use commands::{Args, Commands};
use error::Result;
use kdam::{term, term::Colorizer};
use requestty::symbols;
use std::{io, io::IsTerminal, process};

fn run() -> Result<()> {
    let args = Args::parse();

    term::init(match args.color {
        ColorChoice::Always => true,
        ColorChoice::Auto => io::stderr().is_terminal(),
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
        eprintln!(
            "{}{} {}",
            "error".colorize("bold red"),
            ":".colorize("bold white"),
            e
        );
        process::exit(1);
    }
}

/*
    TODOs

    1. Add resume support
    2. Reduce dependency on ffmpeg

*/
