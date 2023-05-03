mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod merger;
mod mp4parser;
mod playlist;
mod utils;

use clap::Parser;
use commands::{Args, Commands};
use kdam::term::Colorizer;

fn run() -> anyhow::Result<()> {
    match Args::parse().command {
        #[cfg(feature = "chrome")]
        Commands::Collect(args) => args.perform()?,
        Commands::Extract(args) => args.perform()?,
        Commands::Merge(args) => args.perform()?,
        Commands::Save(args) => args.perform()?,
    }

    Ok(())
}

fn main() {
    let mut symbols = requestty::symbols::UNICODE;
    symbols.completed = '•';
    symbols.cross = 'x';
    requestty::symbols::set(symbols);

    if let Err(e) = run() {
        eprintln!("{}: {}", "error".colorize("bold red"), e);
        std::process::exit(1);
    }
}

/*
    TODOs

    1. Add resume support
    2. Create a custom thread pool module
    3. Reduce dependency on anyhow crate
    4. Reduce dependency on ffmpeg
    5. Remove #[allow(dead_code)]

*/
