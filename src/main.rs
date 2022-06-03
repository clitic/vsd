use kdam::term::Colorizer;

fn error(e: anyhow::Error) -> ! {
    println!("{}: {}", "Error".colorize("bold red"), e);
    // println!("{}: {}", "Cause".colorize("bold yellow"), e.root_cause());
    std::process::exit(1);
}

fn main() {
    let mut downloader = vsd::core::DownloadState::new().unwrap_or_else(|e| error(e));
    let segments = downloader.segments().unwrap_or_else(|e| error(e));
    downloader
        .download(&segments, downloader.determine_output())
        .unwrap_or_else(|e| error(e));
}
