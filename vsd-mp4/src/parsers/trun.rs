use crate::{Reader, Result};

pub struct TRUNSample {
    /// The length of the sample in timescale units.
    pub sample_duration: Option<u32>,
    /// The size of the sample in bytes.
    pub sample_size: Option<u32>,
    /// The time since the start of the sample in timescale units. Time
    /// offset is based of the start of the sample. If this value is
    /// missing, the accumulated durations preceeding this time sample will
    /// be used to create the start time.
    pub sample_composition_time_offset: Option<i32>,
}

pub struct TRUNBox {
    /// As per the spec: the number of samples being added in this run;
    pub sample_count: u32,
    /// An array of size sampleCount containing data for each sample
    pub sample_data: Vec<TRUNSample>,
    /// If specified via flags, this indicate the offset of the sample in bytes.
    pub data_offset: Option<u32>,
}

impl TRUNBox {
    /// Parses a TRUN Box.
    pub fn new(reader: &mut Reader, version: u32, flags: u32) -> Result<Self> {
        let sample_count = reader.read_u32()?;
        let mut sample_data = vec![];
        let mut data_offset = None;

        // "data_offset"
        if (flags & 0x000001) != 0 {
            data_offset = Some(reader.read_u32()?);
        }

        // Skip "first_sample_flags" if present.
        if (flags & 0x000004) != 0 {
            reader.skip(4)?;
        }

        for _ in 0..sample_count {
            let mut sample = TRUNSample {
                sample_duration: None,
                sample_size: None,
                sample_composition_time_offset: None,
            };

            // Read "sample duration" if present.
            if (flags & 0x000100) != 0 {
                sample.sample_duration = Some(reader.read_u32()?);
            }

            // Read "sample_size" if present.
            if (flags & 0x000200) != 0 {
                sample.sample_size = Some(reader.read_u32()?);
            }

            // Skip "sample_flags" if present.
            if (flags & 0x000400) != 0 {
                reader.skip(4)?;
            }

            // Read "sample_time_offset" if present.
            if (flags & 0x000800) != 0 {
                sample.sample_composition_time_offset = Some(if version == 0 {
                    reader.read_u32()? as i32
                } else {
                    reader.read_i32()?
                });
            }

            sample_data.push(sample);
        }

        Ok(Self {
            sample_count,
            sample_data,
            data_offset,
        })
    }
}
