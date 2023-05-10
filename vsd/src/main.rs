mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod merger;
mod mp4parser;
mod playlist;
mod update;
mod utils;

use clap::{ColorChoice, Parser};
use commands::{Args, Commands};
use kdam::term::Colorizer;

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.color {
        ColorChoice::Auto => {
            if atty::isnt(atty::Stream::Stdout) {
                kdam::term::set_colorize(false);
            }
        }
        ColorChoice::Never => {
            kdam::term::set_colorize(false);
        }
        _ => (),
    }
    
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
