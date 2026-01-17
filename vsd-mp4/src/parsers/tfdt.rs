use crate::{Reader, Result};

pub struct TFDTBox {
    /// As per the spec: the absolute decode time, measured on the media
    /// timeline, of the first sample in decode order in the track fragment
    pub base_media_decode_time: u64,
}

impl TFDTBox {
    /// Parses a TFDT Box.
    pub fn new(reader: &mut Reader, version: u32) -> Result<Self> {
        Ok(Self {
            base_media_decode_time: if version == 1 {
                reader.read_u64()?
            } else {
                reader.read_u32()? as u64
            },
        })
    }
}
