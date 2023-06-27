//! Mp4 parsers related to some subtitles text processing.

/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/tree/main/lib/text

*/

mod boxes;
mod subtitles;

use subtitles::Cue;

pub use subtitles::Subtitles;

#[cfg(feature = "text-ttml")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-ttml")))]
pub mod ttml_text_parser;

#[cfg(feature = "text-ttml")]
mod mp4_ttml_parser;

#[cfg(feature = "text-ttml")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-ttml")))]
pub use mp4_ttml_parser::Mp4TtmlParser;

#[cfg(feature = "text-vtt")]
mod mp4_vtt_parser;

#[cfg(feature = "text-vtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-vtt")))]
pub use mp4_vtt_parser::Mp4VttParser;
