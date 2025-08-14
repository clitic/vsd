/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/62c8367438d36c08db6440ba32f54223e0367f00/lib/dash/mp4_segment_index_parser.js

*/

//! Mp4 `SIDX` box parser.

use crate::{Error, Mp4Parser, ParsedBox, Result};
use std::sync::{Arc, Mutex};

/// Segment range.
#[derive(Clone)]
pub struct Range {
    pub end: u64,
    pub start: u64,
}

/// Mp4 `SegmentBase@indexRange` parser.
/// `sidx_offset` is the starting byte of sidx box.
pub fn parse(data: &[u8], sidx_offset: u64) -> Result<Vec<Range>> {
    let references = Arc::new(Mutex::new(Vec::new()));
    let references_c = references.clone();

    Mp4Parser::default()
        .full_box(
            "sidx",
            Arc::new(move |mut _box| {
                *references_c.lock().unwrap() = parse_sidx(&mut _box, sidx_offset)?;
                Ok(())
            }),
        )
        .parse(data, false, false)?;

    let references = references.lock().unwrap().to_vec();
    Ok(references)
}

fn parse_sidx(_box: &mut ParsedBox, sidx_offset: u64) -> Result<Vec<Range>> {
    if _box.version.is_none() {
        return Err(Error::new(
            "SIDX is a full box and should have a valid version.",
        ));
    }

    let reader = &mut _box.reader;
    let version = _box.version.unwrap();

    let mut references = Vec::new();

    reader
        .skip(4)
        .map_err(|_| Error::new_read("SIDX box skip reference ID (32 bits)."))?;

    let timescale = reader
        .read_u32()
        .map_err(|_| Error::new_read("SIDX box timescale (u32)."))?;

    if timescale == 0 {
        return Err(Error::new("SIDX box has invalid timescale."));
    }

    let _earliest_presentation_time;
    let first_offset;

    if version == 0 {
        _earliest_presentation_time = reader
            .read_u32()
            .map_err(|_| Error::new_read("SIDX box earliest presentation time (u32)."))?
            as u64;
        first_offset = reader
            .read_u32()
            .map_err(|_| Error::new_read("SIDX box first offset (u32)."))?
            as u64;
    } else {
        _earliest_presentation_time = reader
            .read_u64()
            .map_err(|_| Error::new_read("SIDX box earliest presentation time (u64)."))?;
        first_offset = reader
            .read_u64()
            .map_err(|_| Error::new_read("SIDX box first offset (u64)."))?;
    }

    reader
        .skip(2)
        .map_err(|_| Error::new_read("SIDX box skip reserved (16 bits)."))?;

    let reference_count = reader
        .read_u16()
        .map_err(|_| Error::new_read("SIDX box reference count (u16)."))?;

    // Subtract the presentation time offset
    // let mut unscaled_start_time = earliest_presentation_time;
    let mut start_byte = sidx_offset + _box.size as u64 + first_offset;

    for _ in 0..reference_count {
        // |chunk| is 1 bit for |referenceType|, and 31 bits for |referenceSize|.
        let chunk = reader
            .read_u32()
            .map_err(|_| Error::new_read("SIDX box chunk (u32)."))?;
        let reference_type = (chunk & 0x80000000) >> 31;
        let reference_size = chunk & 0x7FFFFFFF;

        let _subsegment_duration = reader
            .read_u32()
            .map_err(|_| Error::new_read("SIDX box subsegment duration (u32)."))?;

        // Skipping 1 bit for |startsWithSap|, 3 bits for |sapType|, and 28 bits
        // for |sapDelta|.
        reader
            .skip(4)
            .map_err(|_| Error::new_read("SIDX box skip (32 bits)."))?;

        // If |referenceType| is 1 then the reference is to another SIDX.
        // We do not support this.
        if reference_type == 1 {
            return Err(Error::new("hierarchical SIDXs are not supported."));
        }

        // The media timestamps inside the container.
        // let native_start_Time = unscaled_start_time as f64 / timescale as f64;
        // let native_end_Time = (unscaled_start_time as f64 + subsegment_duration as f64) / timescale as f64;

        references.push(Range {
            end: start_byte + reference_size as u64 - 1,
            start: start_byte,
        });

        // unscaled_start_time += subsegment_duration as u64;
        start_byte += reference_size as u64;
    }

    _box.parser.stop();
    Ok(references)
}
