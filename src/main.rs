use anyhow::Error;
use clap::Parser;
use kdam::term::Colorizer;
use kdam::RichProgress;
use std::sync::{Arc, Mutex};

fn error(e: Error) -> ! {
    println!("{}: {}", "error".colorize("bold red"), e);
    std::process::exit(1);
}

fn error_progress_bar(e: Error, _pb: &Arc<Mutex<RichProgress>>) -> ! {
    println!("\n{}: {}", "error".colorize("bold red"), e);
    std::process::exit(1);
}

fn main() {
    let args = vsd::Args::parse();

    if args.capture {
        vsd::chrome::capture(&args.input, args.headless).unwrap_or_else(|e| error(e));
    } else if args.collect {
        vsd::chrome::collect(&args.input, args.headless, args.build).unwrap_or_else(|e| error(e));
    } else {
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
