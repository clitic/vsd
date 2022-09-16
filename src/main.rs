use anyhow::Error;
use clap::Parser;
use kdam::term::Colorizer;
use kdam::RichProgress;
use std::sync::{Arc, Mutex};
use vsd::args::Commands;

fn error(e: Error) -> ! {
    eprintln!("{}: {}", "error".colorize("bold red"), e);
    std::process::exit(1);
}

fn error_progress_bar(e: Error, _pb: &Arc<Mutex<RichProgress>>) -> ! {
    eprintln!("\n{}: {}", "error".colorize("bold red"), e);
    std::process::exit(1);
}

fn main() {
    match vsd::args::Args::parse().command {
        Commands::Capture(args) => {
            vsd::chrome::capture(&args.url, args.headless).unwrap_or_else(|e| error(e))
        }
        Commands::Collect(args) => {
            vsd::chrome::collect(&args.url, args.headless, args.build).unwrap_or_else(|e| error(e))
        }
        Commands::Decrypt(args) => args.perform().unwrap_or_else(|e| error(e)),
        Commands::Extract(args) => args.perform().unwrap_or_else(|e| error(e)),
        Commands::Merge(args) => args.perform().unwrap_or_else(|e| error(e)),
        Commands::Save(args) => {
            let mut downloader = vsd::DownloadState::new(args).unwrap_or_else(|e| error(e));
            downloader.fetch_playlists().unwrap_or_else(|e| error(e));
            downloader.check_segments().unwrap_or_else(|e| error(e));
            downloader
                .download()
                .unwrap_or_else(|e| error_progress_bar(e, &downloader.pb));
            downloader
                .progress
                .transmux_trancode(downloader.args.output.clone(), downloader.args.alternative)
                .unwrap_or_else(|e| error(e));
        }
    }
}
