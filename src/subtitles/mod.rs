// REFERENCES: https://github.com/nilaoda/Mp4SubtitleParser

mod mp4parser;
mod reader;
mod vtt;

use mp4parser::{MP4Parser, Sample, TFHD, TRUN};
use reader::Reader;

pub use vtt::{Subtitles, MP4VTT};

// fn main() {
//     let parser = app::MP4VTT::from_init(
//         include_bytes!("../sub/sub/init.mp4"),
//         &[
//             include_bytes!("../sub/sub/segment-1.0001.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0002.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0003.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0004.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0005.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0006.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0007.m4s").to_vec(),
//             include_bytes!("../sub/sub/segment-1.0008.m4s").to_vec(),
//         ],
//     )
//     .unwrap();

//     println!("{}", parser.to_subtitles().to_vtt());
// }
