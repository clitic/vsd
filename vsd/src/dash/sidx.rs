// Parsing sidx boxes in ISOBMFF containers and WebM Cue information.
//
// Manifests to test with:
//  (WebM) https://storage.googleapis.com/shaka-demo-assets/sintel/dash.mpd
//  (MP4)  https://turtle-tube.appspot.com/t/t2/dash.mpd


use std::io::{Cursor, Read};
use byteorder::{BigEndian, ReadBytesExt};


// A Segment Index Box provides a compact index of one media stream within the media segment to which 
// it applies.
#[derive(Debug, Clone, PartialEq)]
pub struct SidxBox {
    pub version: u8,
    pub flags: u32,   // actually only u24
    pub reference_id: u32,
    pub timescale: u32,
    pub earliest_presentation_time: u64,
    pub first_offset: u64,
    pub reference_count: u16,
    pub references: Vec<SidxReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidxReference {
    pub reference_type: u8,
    pub referenced_size: u32,
    pub subsegment_duration: u32,
    pub starts_with_sap: u8,  // (actually a boolean)
    pub sap_type: u8,
    pub sap_delta_time: u32,
}


impl SidxBox {
    pub fn parse(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut rdr = Cursor::new(data);
        let _box_size = rdr.read_u32::<BigEndian>()?;
        let mut box_header = [0u8; 4];
        if rdr.read_exact(&mut box_header).is_err() {
            return Err("reading box header".into());
        }
        if !box_header.eq(b"sidx") {
            return Err("expecting sidx BMFF header".into());
        }
        let version = rdr.read_u8()?;
        let flags = rdr.read_u24::<BigEndian>()?;
        let reference_id = rdr.read_u32::<BigEndian>()?;
        let timescale = rdr.read_u32::<BigEndian>()?;
        let earliest_presentation_time = if version == 0 {
            u64::from(rdr.read_u32::<BigEndian>()?)
        } else {
            rdr.read_u64::<BigEndian>()?
        };
        let first_offset = if version == 0 {
            u64::from(rdr.read_u32::<BigEndian>()?)
        } else {
            rdr.read_u64::<BigEndian>()?
        };
        let _reserved = rdr.read_u16::<BigEndian>()?;
        let reference_count = rdr.read_u16::<BigEndian>()?;
        let mut references = Vec::with_capacity(reference_count as usize);
        for _ in 0..reference_count {
            // chunk is 1 bit for reference_type, and 31 bits for referenced_size.
            let chunk = rdr.read_u32::<BigEndian>()?;
            // Reference_type = 1 means a reference to another sidx (hierarchical sidx)
            let reference_type = ((chunk & 0x80000000) >> 31) as u8;
            // if reference_type != 0 {
            //     warn!("Don't know how to handle hierarchical sidx");
            // }
            let referenced_size = chunk & 0x7FFFFFFF;
            let subsegment_duration = rdr.read_u32::<BigEndian>()?;
            let fields = rdr.read_u32::<BigEndian>()?;
            let starts_with_sap = if (fields >> 31) == 1 { 1 } else { 0 };
            let sap_type = ((fields >> 28) & 0b0111) as u8;
            let sap_delta_time = fields & !(0b1111 << 28);

            references.push(SidxReference {
                reference_type,
                referenced_size,
                subsegment_duration,
                starts_with_sap,
                sap_type,
                sap_delta_time,
            });
        }
        Ok(SidxBox {
            version,
            flags,
            reference_id,
            timescale,
            earliest_presentation_time,
            first_offset,
            reference_count,
            references,
        })
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct SegmentChunk {
    pub start: u64,
    pub end: u64
}

pub fn from_isobmff_sidx(data: &[u8], index_start: u64) -> Result<Vec<SegmentChunk>, Box<dyn std::error::Error>> {
    let mut chunks = Vec::new();
    let sidx = SidxBox::parse(data)?;
    let mut current_pos = index_start;
    for sref in sidx.references {
        let start = current_pos;
        let end = current_pos - 1 + sref.referenced_size as u64;
        chunks.push(SegmentChunk{ start, end });
        current_pos += sref.referenced_size as u64;
    }
    Ok(chunks)
}
