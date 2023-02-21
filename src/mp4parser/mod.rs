mod parser;
mod reader;
mod boxes;

pub mod mp4init;
pub mod subtitles;

use parser::{
    alldata, children, sample_description, type_to_string, MP4Parser, Sample, ParsedTFHDBox, TRUN,
};
use reader::Reader;
