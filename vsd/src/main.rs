use clap::Parser;
use log::error;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // FIX - cursor hide unhide
    // eprint!("\x1B[?25l");

    if let Err(e) = vsd::Args::parse().execute().await {
        error!("{}", e);
        std::process::exit(1);
    }

    // eprint!("\x1B[?25h")
}
