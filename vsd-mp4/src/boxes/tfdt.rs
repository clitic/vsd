use crate::{ParsedBox, Result};

pub struct TfdtBox {
    /// As per the spec: the absolute decode time, measured on the media
    /// timeline, of the first sample in decode order in the track fragment
    pub base_media_decode_time: u64,
}

impl TfdtBox {
    /// Parses a TFDT Box.
    pub fn new(box_: &mut ParsedBox) -> Result<Self> {
        let reader = &mut box_.reader;
        let version = box_.version.unwrap();

        Ok(Self {
            base_media_decode_time: if version == 1 {
                reader.read_u64()?
            } else {
                reader.read_u32()? as u64
            },
        })
    }
}
