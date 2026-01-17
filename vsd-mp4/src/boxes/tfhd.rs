use crate::{ParsedBox, Result};

pub struct TfhdBox {
    /// As per the spec: an integer that uniquely identifies this
    /// track over the entire life‐time of this presentation
    pub track_id: u32,
    /// If specified via flags, this overrides the default sample
    /// duration in the Track Extends Box for this fragment
    pub default_sample_duration: Option<u32>,
    /// If specified via flags, this overrides the default sample
    /// size in the Track Extends Box for this fragment
    pub default_sample_size: Option<u32>,
    /// If specified via flags, this indicate the base data offset
    pub base_data_offset: Option<u64>,
}

impl TfhdBox {
    /// Parses a TFHD Box.
    pub fn new(box_: &mut ParsedBox) -> Result<Self> {
        let reader = &mut box_.reader;
        let flags = box_.flags.unwrap();

        let mut default_sample_duration = None;
        let mut default_sample_size = None;
        let mut base_data_offset = None;

        let track_id = reader.read_u32()?;

        // Skip "base_data_offset" if present.
        if (flags & 0x000001) != 0 {
            base_data_offset = Some(reader.read_u64()?);
        }

        // Skip "sample_description_index" if present.
        if (flags & 0x000002) != 0 {
            reader.skip(4)?;
        }

        // Read "default_sample_duration" if present.
        if (flags & 0x000008) != 0 {
            default_sample_duration = Some(reader.read_u32()?);
        }

        // Read "default_sample_size" if present.
        if (flags & 0x000010) != 0 {
            default_sample_size = Some(reader.read_u32()?);
        }

        Ok(Self {
            track_id,
            default_sample_duration,
            default_sample_size,
            base_data_offset,
        })
    }
}
