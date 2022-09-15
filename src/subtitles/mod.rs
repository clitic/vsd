// REFERENCES: https://github.com/nilaoda/Mp4SubtitleParser

mod cue;
mod mp4parser;
mod reader;
mod ttml;
mod mp4subtitles;

use cue::Cue;
use mp4parser::{MP4Parser, Sample, TFHD, TRUN};
use reader::Reader;

pub use cue::Subtitles;
pub use mp4subtitles::MP4Subtitles;
