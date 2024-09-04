/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/d465942c4393e6c891d6a230bea90a44d90cc70b/lib/util/mp4_box_parsers.js

*/

use crate::{Error, Reader, Result};

pub(super) struct TFHDBox {
    /// As per the spec: an integer that uniquely identifies this
    /// track over the entire life‐time of this presentation
    pub(super) _track_id: u32,
    /// If specified via flags, this overrides the default sample
    /// duration in the Track Extends Box for this fragment
    pub(super) default_sample_duration: Option<u32>,
    /// If specified via flags, this overrides the default sample
    /// size in the Track Extends Box for this fragment
    pub(super) _default_sample_size: Option<u32>,
    /// If specified via flags, this indicate the base data offset
    pub(super) _base_data_offset: Option<u64>,
}

impl TFHDBox {
    /// Parses a TFHD Box.
    pub(super) fn parse(reader: &mut Reader, flags: u32) -> Result<Self> {
        let mut default_sample_duration = None;
        let mut default_sample_size = None;
        let mut base_data_offset = None;

        let track_id = reader
            .read_u32()
            .map_err(|_| Error::new_read_err("TFHD box track id (u32)"))?;

        // Skip "base_data_offset" if present.
        if (flags & 0x000001) != 0 {
            base_data_offset = Some(
                reader
                    .read_u64()
                    .map_err(|_| Error::new_read_err("TFHD box data offset (u64)"))?,
            );
        }

        // Skip "sample_description_index" if present.
        if (flags & 0x000002) != 0 {
            reader.skip(4).map_err(|_| {
                Error::new_read_err("TFHD box sample description index data (4 bytes)")
            })?;
        }

        // Read "default_sample_duration" if present.
        if (flags & 0x000008) != 0 {
            default_sample_duration = Some(
                reader
                    .read_u32()
                    .map_err(|_| Error::new_read_err("TFHD box default sample duration (u32)"))?,
            );
        }

        // Read "default_sample_size" if present.
        if (flags & 0x000010) != 0 {
            default_sample_size = Some(
                reader
                    .read_u32()
                    .map_err(|_| Error::new_read_err("TFHD box default sample size (u32)"))?,
            );
        }

        Ok(Self {
            _track_id: track_id,
            default_sample_duration,
            _default_sample_size: default_sample_size,
            _base_data_offset: base_data_offset,
        })
    }
}

pub(super) struct TFDTBox {
    /// As per the spec: the absolute decode time, measured on the media
    /// timeline, of the first sample in decode order in the track fragment
    pub(super) base_media_decode_time: u64,
}

impl TFDTBox {
    /// Parses a TFDT Box.
    pub(super) fn parse(reader: &mut Reader, version: u32) -> Result<Self> {
        Ok(Self {
            base_media_decode_time: if version == 1 {
                reader
                    .read_u64()
                    .map_err(|_| Error::new_read_err("TFDT box base media decode time (u64)"))?
            } else {
                reader
                    .read_u32()
                    .map_err(|_| Error::new_read_err("TFDT box base media decode time (u32)"))?
                    as u64
            },
        })
    }
}

pub(super) struct MDHDBox {
    /// As per the spec: an integer that specifies the time‐scale for this media;
    /// this is the number of time units that pass in one second
    pub(super) timescale: u32,
    /// Language code for this media
    pub(super) _language: String,
}

impl MDHDBox {
    /// Parses a MDHD Box.
    pub(super) fn parse(reader: &mut Reader, version: u32) -> Result<Self> {
        if version == 1 {
            reader
                .skip(8)
                .map_err(|_| Error::new_read_err("MDHD box creation time data (8 bytes)"))?;
            reader
                .skip(8)
                .map_err(|_| Error::new_read_err("MDHD box modification time data (8 bytes)"))?;
        } else {
            reader
                .skip(4)
                .map_err(|_| Error::new_read_err("MDHD box creation time data (4 bytes)"))?;
            reader
                .skip(4)
                .map_err(|_| Error::new_read_err("MDHD box modification time data (4 bytes)"))?;
        }

        let timescale = reader
            .read_u32()
            .map_err(|_| Error::new_read_err("MDHD box timescale (u32)"))?;

        reader
            .skip(4)
            .map_err(|_| Error::new_read_err("MDHD box duration data (4 bytes)"))?;

        let language = reader
            .read_u16()
            .map_err(|_| Error::new_read_err("MDHD box language data (u16)"))?;

        // language is stored as an ISO-639-2/T code in an array of three
        // 5-bit fields each field is the packed difference between its ASCII
        // value and 0x60
        let language_string = String::from_utf16(&[
            (language >> 10) + 0x60,
            ((language & 0x03c0) >> 5) + 0x60,
            (language & 0x1f) + 0x60,
        ])
        .map_err(|_| Error::new_decode_err("MDHD box language as vaild utf-16 data"))?;

        Ok(Self {
            timescale,
            _language: language_string,
        })
    }
}

pub(super) struct TRUNBox {
    /// As per the spec: the number of samples being added in this run;
    pub(super) _sample_count: u32,
    /// An array of size sampleCount containing data for each sample
    pub(super) sample_data: Vec<TRUNSample>,
    /// If specified via flags, this indicate the offset of the sample in bytes.
    pub(super) _data_offset: Option<u32>,
}

impl TRUNBox {
    /// Parses a TRUN Box.
    pub(super) fn parse(reader: &mut Reader, version: u32, flags: u32) -> Result<Self> {
        let sample_count = reader
            .read_u32()
            .map_err(|_| Error::new_read_err("TRUN box sample count (u32)"))?;
        let mut sample_data = vec![];
        let mut data_offset = None;

        // "data_offset"
        if (flags & 0x000001) != 0 {
            data_offset = Some(
                reader
                    .read_u32()
                    .map_err(|_| Error::new_read_err("TRUN box data offset (u32)"))?,
            );
        }

        // Skip "first_sample_flags" if present.
        if (flags & 0x000004) != 0 {
            reader
                .skip(4)
                .map_err(|_| Error::new_read_err("TRUN box first sample flags (4 bytes)"))?;
        }

        for _ in 0..sample_count {
            let mut sample = TRUNSample {
                sample_duration: None,
                sample_size: None,
                sample_composition_time_offset: None,
            };

            // Read "sample duration" if present.
            if (flags & 0x000100) != 0 {
                sample.sample_duration = Some(
                    reader
                        .read_u32()
                        .map_err(|_| Error::new_read_err("TRUN box sample duration (u32)"))?,
                );
            }

            // Read "sample_size" if present.
            if (flags & 0x000200) != 0 {
                sample.sample_size = Some(
                    reader
                        .read_u32()
                        .map_err(|_| Error::new_read_err("TRUN box sample size (u32)"))?,
                );
            }

            // Skip "sample_flags" if present.
            if (flags & 0x000400) != 0 {
                reader
                    .skip(4)
                    .map_err(|_| Error::new_read_err("TRUN box sample flags (u32)"))?;
            }

            // Read "sample_time_offset" if present.
            if (flags & 0x000800) != 0 {
                sample.sample_composition_time_offset = Some(if version == 0 {
                    reader
                        .read_u32()
                        .map_err(|_| Error::new_read_err("TRUN box sample time offset (u32)"))?
                        as i32
                } else {
                    reader
                        .read_i32()
                        .map_err(|_| Error::new_read_err("TRUN box sample time offset (i32)"))?
                });
            }

            sample_data.push(sample);
        }

        Ok(Self {
            _sample_count: sample_count,
            sample_data,
            _data_offset: data_offset,
        })
    }
}

pub(super) struct TRUNSample {
    /// The length of the sample in timescale units.
    pub(super) sample_duration: Option<u32>,
    /// The size of the sample in bytes.
    pub(super) sample_size: Option<u32>,
    /// The time since the start of the sample in timescale units. Time
    /// offset is based of the start of the sample. If this value is
    /// missing, the accumulated durations preceeding this time sample will
    /// be used to create the start time.
    pub(super) sample_composition_time_offset: Option<i32>,
}
