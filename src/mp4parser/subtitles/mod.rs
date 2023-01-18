/*
    REFERENCES
    ----------

    1. https://github.com/xhlove/dash-subtitle-extractor
    2. https://github.com/nilaoda/Mp4SubtitleParser

*/

mod cue;
mod ttml;
mod mp4subtitles;

use cue::Cue;

pub use cue::Subtitles;
pub use mp4subtitles::MP4Subtitles;
