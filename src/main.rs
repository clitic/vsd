use kdam::term::Colorizer;

fn error(e: anyhow::Error) -> ! {
    println!("{}: {}", "Error".colorize("bold red"), e);
    std::process::exit(1);
}

fn main() {
    let mut downloader = vsd::core::DownloadState::new().unwrap_or_else(|e| error(e));
    let segments = downloader.segments().unwrap_or_else(|e| error(e));
    downloader
        .download(&segments, downloader.tempfile())
        .unwrap_or_else(|e| error(e));
    downloader.transmux_trancode().unwrap_or_else(|e| error(e));
}
