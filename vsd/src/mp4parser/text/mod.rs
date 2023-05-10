/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/tree/main/lib/text

*/

mod cue;
mod mp4_ttml_parser;
mod mp4_vtt_parser;

pub mod ttml_text_parser;

pub use cue::{Cue, Subtitles};
pub use mp4_ttml_parser::Mp4TtmlParser;
pub use mp4_vtt_parser::Mp4VttParser;
