mod parser;
mod reader;

pub mod mp4init;
pub mod subtitles;

use parser::{
    alldata, children, sample_description, type_to_string, MP4Parser, Sample, TFHD, TRUN,
};
use reader::Reader;
