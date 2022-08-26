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

    // let mpd = vsd::dash::parse(include_bytes!("../11331.xml")).unwrap();
    
    // let mut stdout = std::io::stdout();
    // vsd::dash::to_m3u8_as_master(&mpd).write_to(&mut stdout).unwrap();
    // println!();
    // vsd::dash::to_m3u8_as_media(&mpd, "https://github.com", "dash://6").unwrap().write_to(&mut stdout).unwrap();
}
