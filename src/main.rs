use anyhow::Error;
use clap::Parser;
use kdam::term::Colorizer;

fn error(e: Error) -> ! {
    println!("{}: {}", "Error".colorize("bold red"), e);
    std::process::exit(1);
}

fn main() {
    let args = vsd::Args::parse();
    
    if args.capture {
        vsd::chrome::capture(&args.input, args.headless).unwrap_or_else(|e| error(e));
    } else if args.collect {
        vsd::chrome::collect(&args.input, args.headless, args.build).unwrap_or_else(|e| error(e));
    } else {
        let mut downloader = vsd::core::DownloadState::new(args).unwrap_or_else(|e| error(e));
        let segments = downloader.playlist().unwrap_or_else(|e| error(e));
        downloader
            .download(&segments, downloader.args.tempfile())
            .unwrap_or_else(|e| error(e));
        downloader.transmux_trancode().unwrap_or_else(|e| error(e));
    }


    // let mpd = vsd::dash::parse(include_bytes!("../11331.xml")).unwrap();

    // let mut stdout = std::io::stdout();
    // vsd::dash::to_m3u8_as_master(&mpd).write_to(&mut stdout).unwrap();
    // println!();
    // vsd::dash::to_m3u8_as_media(&mpd, "https://github.com", "dash://preiod.0.adaptation-set.0.representation.5").unwrap().write_to(&mut stdout).unwrap();
}
