// REFERENCES: https://github.com/nilaoda/Mp4SubtitleParser

mod mp4parser;
mod reader;
mod vtt;

use mp4parser::{MP4Parser, Sample, TFHD, TRUN};
use reader::Reader;

pub use vtt::{Subtitles, MP4VTT};
