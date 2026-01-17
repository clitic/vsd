use crate::{ParsedBox, Result};

pub struct MdhdBox {
    /// As per the spec: an integer that specifies the time‐scale for this media;
    /// this is the number of time units that pass in one second
    pub timescale: u32,
    /// Language code for this media
    pub language: String,
}

impl MdhdBox {
    /// Parses a MDHD Box.
    pub fn new(box_: &mut ParsedBox) -> Result<Self> {
        let reader = &mut box_.reader;
        let version = box_.version.unwrap();

        if version == 1 {
            reader.skip(8)?;
            reader.skip(8)?;
        } else {
            reader.skip(4)?;
            reader.skip(4)?;
        }

        let timescale = reader.read_u32()?;

        reader.skip(4)?;

        let language = reader.read_u16()?;

        // language is stored as an ISO-639-2/T code in an array of three
        // 5-bit fields each field is the packed difference between its ASCII
        // value and 0x60
        let language_string = String::from_utf16(&[
            (language >> 10) + 0x60,
            ((language & 0x03c0) >> 5) + 0x60,
            (language & 0x1f) + 0x60,
        ])?;

        Ok(Self {
            timescale,
            language: language_string,
        })
    }
}
