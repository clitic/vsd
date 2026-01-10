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

use clap::Parser;
use commands::Args;
use log::error;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // FIX - cursor hide unhide
    eprint!("\x1B[?25l");

    if let Err(e) = Args::parse().execute().await {
        error!("{}", e);
        std::process::exit(1);
    }

    eprint!("\x1B[?25h")
}
