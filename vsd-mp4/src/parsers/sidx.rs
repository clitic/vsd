/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/62c8367438d36c08db6440ba32f54223e0367f00/lib/dash/mp4_segment_index_parser.js

*/

//! Mp4 `SIDX` box parser.

use crate::{Mp4Parser, ParsedBox, Result, bail};
use std::{cell::RefCell, rc::Rc};

/// Segment range.
pub struct SidxRange {
    pub end: u64,
    pub start: u64,
}

/// Mp4 `SegmentBase@indexRange` parser.
/// `sidx_offset` is the starting byte of sidx box.
pub struct SidxBox {
    offset: u64,
    ranges: Rc<RefCell<Vec<SidxRange>>>,
}

impl SidxBox {
    pub fn new(offset: u64) -> Self {
        Self {
            offset,
            ranges: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn parse(self, data: &[u8]) -> Result<Vec<SidxRange>> {
        let offset = self.offset;
        let references = self.ranges.clone();

        Mp4Parser::new()
            .full_box("sidx", move |mut box_| {
                *references.borrow_mut() = Self::parse_box(&mut box_, offset)?;
                Ok(())
            })
            .parse(data, false, false)?;

        Ok(self.ranges.take())
    }

    fn parse_box(box_: &mut ParsedBox, offset: u64) -> Result<Vec<SidxRange>> {
        if box_.version.is_none() {
            bail!("SIDX is a full box and should have a valid version.");
        }

        let reader = &mut box_.reader;
        let version = box_.version.unwrap();

        let mut references = Vec::new();

        reader.skip(4)?;

        let timescale = reader.read_u32()?;

        if timescale == 0 {
            bail!("SIDX box has invalid timescale.");
        }

        let _earliest_presentation_time;
        let first_offset;

        if version == 0 {
            _earliest_presentation_time = reader.read_u32()? as u64;
            first_offset = reader.read_u32()? as u64;
        } else {
            _earliest_presentation_time = reader.read_u64()?;
            first_offset = reader.read_u64()?;
        }

        reader.skip(2)?;

        let reference_count = reader.read_u16()?;

        // Subtract the presentation time offset
        // let mut unscaled_start_time = earliest_presentation_time;
        let mut start_byte = offset + box_.size as u64 + first_offset;

        for _ in 0..reference_count {
            // |chunk| is 1 bit for |referenceType|, and 31 bits for |referenceSize|.
            let chunk = reader.read_u32()?;
            let reference_type = (chunk & 0x80000000) >> 31;
            let reference_size = chunk & 0x7FFFFFFF;

            let _subsegment_duration = reader.read_u32()?;

            // Skipping 1 bit for |startsWithSap|, 3 bits for |sapType|, and 28 bits
            // for |sapDelta|.
            reader.skip(4)?;

            // If |referenceType| is 1 then the reference is to another SIDX.
            // We do not support this.
            if reference_type == 1 {
                bail!("Hierarchical SIDXs are not supported.");
            }

            // The media timestamps inside the container.
            // let native_start_Time = unscaled_start_time as f64 / timescale as f64;
            // let native_end_Time = (unscaled_start_time as f64 + subsegment_duration as f64) / timescale as f64;

            references.push(SidxRange {
                end: start_byte + reference_size as u64 - 1,
                start: start_byte,
            });

            // unscaled_start_time += subsegment_duration as u64;
            start_byte += reference_size as u64;
        }

        box_.parser.stop();
        Ok(references)
    }
}
