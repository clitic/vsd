// use kdam::term::Colorizer;

// fn error(e: anyhow::Error) {
//     println!("{} {}", "Error:".colorize("bold red"), e);
//     std::process::exit(1);
// }

fn main() {
    let mut downloader = vsd::core::DownloadState::new().unwrap();
    let segments = downloader.segments().unwrap();
    downloader.download(&segments, downloader.determine_output()).unwrap();
}
