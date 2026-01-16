/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/util/mp4_box_parsers.js
    2. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/externs/shaka/mp4_parser.js

*/

use crate::{Error, Reader};
use std::{collections::HashMap, sync::Arc};

/// `Result` type returned when parsing an mp4 file.
pub type HandlerResult = Result<(), Error>;
/// Callback type for parsing an mp4 file.
pub type CallbackType = Arc<dyn Fn(ParsedBox) -> HandlerResult>;

/// Mp4 file parser.
#[derive(Clone)]
pub struct Mp4Parser {
    pub headers: HashMap<usize, BoxType>,
    pub box_definitions: HashMap<usize, CallbackType>,
    pub done: bool,
}

impl Mp4Parser {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
            box_definitions: HashMap::new(),
            done: false,
        }
    }

    /// Declare a box type as a Basic Box.
    pub fn base_box(mut self, type_: &str, definition: CallbackType) -> Self {
        let type_code = type_from_string(type_);
        self.headers.insert(type_code, BoxType::BasicBox);
        self.box_definitions.insert(type_code, definition);
        self
    }

    /// Declare a box type as a Full Box.
    pub fn full_box(mut self, type_: &str, definition: CallbackType) -> Self {
        let type_code = type_from_string(type_);
        self.headers.insert(type_code, BoxType::FullBox);
        self.box_definitions.insert(type_code, definition);
        self
    }

    /// Stop parsing. Useful for extracting information from partial segments and
    /// avoiding an out-of-bounds error once you find what you are looking for.
    pub fn stop(&mut self) {
        self.done = true;
    }

    /// Parse the given data using the added callbacks.
    ///
    /// # Arguments
    ///
    /// - `partial_okay` - If true, allow reading partial payloads
    ///   from some boxes. If the goal is a child box, we can sometimes find it
    ///   without enough data to find all child boxes.
    /// - `stop_on_partial` - If true, stop reading if an incomplete
    ///   box is detected.
    pub fn parse(
        &mut self,
        data: &[u8],
        partial_okay: bool,
        stop_on_partial: bool,
    ) -> HandlerResult {
        let mut reader = Reader::new_big_endian(data);

        self.done = false;

        while reader.has_more_data() && !self.done {
            self.parse_next(0, &mut reader, partial_okay, stop_on_partial)?;
        }

        Ok(())
    }

    /// Parse the next box on the current level.
    ///
    /// # Arguments
    ///
    /// - `abs_start` - The absolute start position in the original
    ///   byte array.
    /// - `partial_okay` - If true, allow reading partial payloads
    ///   from some boxes. If the goal is a child box, we can sometimes find it
    ///   without enough data to find all child boxes.
    /// - `stop_on_partial` - If true, stop reading if an incomplete
    ///   box is detected.
    fn parse_next(
        &mut self,
        abs_start: u64,
        reader: &mut Reader,
        partial_okay: bool,
        stop_on_partial: bool,
    ) -> HandlerResult {
        let start = reader.get_position();

        // size(4 bytes) + type(4 bytes) = 8 bytes
        if stop_on_partial && start + 8 > reader.get_length() {
            self.done = true;
            return Ok(());
        }

        let mut size = reader.read_u32()? as u64;
        let type_ = reader.read_u32()? as usize;
        let name = type_to_string(type_)?;
        let mut has_64_bit_size = false;
        // println!("Parsing MP4 box {}", name);

        match size {
            0 => size = reader.get_length() - start,
            1 => {
                if stop_on_partial && reader.get_position() + 8 > reader.get_length() {
                    self.done = true;
                    return Ok(());
                }
                size = reader.read_u64()?;
                has_64_bit_size = true;
            }
            _ => (),
        }

        let box_definition = self.box_definitions.get(&type_);

        if let Some(box_definition) = box_definition {
            let mut version = None;
            let mut flags = None;

            if *self.headers.get(&type_).unwrap() == BoxType::FullBox {
                if stop_on_partial && reader.get_position() + 4 > reader.get_length() {
                    self.done = true;
                    return Ok(());
                }

                let version_and_flags = reader.read_u32()?;
                version = Some(version_and_flags >> 24);
                flags = Some(version_and_flags & 0xFFFFFF);
            }

            // Read the whole payload so that the current level can be safely read
            // regardless of how the payload is parsed.
            let mut end = start + size;

            if partial_okay && end > reader.get_length() {
                // For partial reads, truncate the payload if we must.
                end = reader.get_length();
            }

            if stop_on_partial && end > reader.get_length() {
                self.done = true;
                return Ok(());
            }

            let payload_size = end - reader.get_position();
            let payload = if payload_size > 0 {
                reader.read_bytes_u8(payload_size as usize)?
            } else {
                Vec::with_capacity(0)
            };

            let payload_reader = Reader::new_big_endian(&payload);

            let box_ = ParsedBox {
                name,
                parser: self.clone(),
                partial_okay,
                stop_on_partial,
                version,
                flags,
                reader: payload_reader,
                size: size as usize,
                start: start + abs_start,
                has_64_bit_size,
            };

            box_definition(box_)?;
        } else {
            // Move the read head to be at the end of the box.
            // If the box is longer than the remaining parts of the file, e.g. the
            // mp4 is improperly formatted, or this was a partial range request that
            // ended in the middle of a box, just skip to the end.
            let skip_length = (start + size - reader.get_position())
                .min(reader.get_length() - reader.get_position());
            reader.skip(skip_length)?;
        }

        Ok(())
    }
}

// CALLBACKS

/// A callback that tells the Mp4 parser to treat the body of a box as a series
/// of boxes. The number of boxes is limited by the size of the parent box.
pub fn children(mut box_: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = box_.header_size();

    while box_.reader.has_more_data() && !box_.parser.done {
        box_.parser.parse_next(
            box_.start + header_size,
            &mut box_.reader,
            box_.partial_okay,
            box_.stop_on_partial,
        )?;
    }

    Ok(())
}

/// A callback that tells the Mp4 parser to treat the body of a box as a sample
/// description. A sample description box has a fixed number of children. The
/// number of children is represented by a 4 byte unsigned integer. Each child
/// is a box.
pub fn sample_description(mut box_: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = box_.header_size();
    let count = box_.reader.read_u32()?;

    for _ in 0..count {
        box_.parser.parse_next(
            box_.start + header_size,
            &mut box_.reader,
            box_.partial_okay,
            box_.stop_on_partial,
        )?;

        if box_.parser.done {
            break;
        }
    }

    Ok(())
}

/// A callback that tells the Mp4 parser to treat the body of a box as a visual
/// sample entry. A visual sample entry has some fixed-sized fields
/// describing the video codec parameters, followed by an arbitrary number of
/// appended children. Each child is a box.
pub fn visual_sample_entry(mut box_: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = box_.header_size();

    // Skip 6 reserved bytes.
    // Skip 2-byte data reference index.
    // Skip 16 more reserved bytes.
    // Skip 4 bytes for width/height.
    // Skip 8 bytes for horizontal/vertical resolution.
    // Skip 4 more reserved bytes (0)
    // Skip 2-byte frame count.
    // Skip 32-byte compressor name (length byte, then name, then 0-padding).
    // Skip 2-byte depth.
    // Skip 2 more reserved bytes (0xff)
    // 78 bytes total.
    // See also https://github.com/shaka-project/shaka-packager/blob/d5ca6e84/packager/media/formats/mp4/box_definitions.cc#L1544
    box_.reader.skip(78)?;

    while box_.reader.has_more_data() && !box_.parser.done {
        box_.parser.parse_next(
            box_.start + header_size,
            &mut box_.reader,
            box_.partial_okay,
            box_.stop_on_partial,
        )?;
    }

    Ok(())
}

/// A callback that tells the Mp4 parser to treat the body of a box as a audio
/// sample entry.  A audio sample entry has some fixed-sized fields
/// describing the audio codec parameters, followed by an arbitrary number of
/// ppended children.  Each child is a box.
pub fn audio_sample_entry(mut box_: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = box_.header_size();

    // 6 bytes reserved
    // 2 bytes data reference index
    box_.reader.skip(8)?;

    // 2 bytes version
    let version = box_.reader.read_u16()?;
    // 2 bytes revision (0, could be ignored)
    // 4 bytes reserved
    box_.reader.skip(6)?;

    if version == 2 {
        // 16 bytes hard-coded values with no comments
        // 8 bytes sample rate
        // 4 bytes channel count
        // 4 bytes hard-coded values with no comments
        // 4 bytes bits per sample
        // 4 bytes lpcm flags
        // 4 bytes sample size
        // 4 bytes samples per packet
        box_.reader.skip(48)?;
    } else {
        // 2 bytes channel count
        // 2 bytes bits per sample
        // 2 bytes compression ID
        // 2 bytes packet size
        // 2 bytes sample rate
        // 2 byte reserved
        box_.reader.skip(12)?;
    }

    if version == 1 {
        // 4 bytes samples per packet
        // 4 bytes bytes per packet
        // 4 bytes bytes per frame
        // 4 bytes bytes per sample
        box_.reader.skip(16)?;
    }

    while box_.reader.has_more_data() && !box_.parser.done {
        box_.parser.parse_next(
            box_.start + header_size,
            &mut box_.reader,
            box_.partial_okay,
            box_.stop_on_partial,
        )?;
    }

    Ok(())
}

/// Create a callback that tells the Mp4 parser to treat the body of a box as a
/// binary blob and to parse the body's contents using the provided callback.
#[allow(clippy::arc_with_non_send_sync)]
pub fn alldata(callback: Arc<dyn Fn(Vec<u8>) -> HandlerResult>) -> CallbackType {
    Arc::new(move |mut _box| {
        let all = _box.reader.get_length() - _box.reader.get_position();
        callback(_box.reader.read_bytes_u8(all as usize)?)
    })
}

// UTILS

/// Convert an ascii string name to the integer type for a box.
/// The name must be four characters long.
pub fn type_from_string(name: &str) -> usize {
    assert!(name.len() == 4, "MP4 box names must be 4 characters long");

    let mut code = 0;

    for chr in name.chars() {
        code = (code << 8) | chr as usize;
    }

    code
}

/// Convert an integer type from a box into an ascii string name.
/// Useful for debugging.
pub fn type_to_string(type_: usize) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(vec![
        ((type_ >> 24) & 0xff) as u8,
        ((type_ >> 16) & 0xff) as u8,
        ((type_ >> 8) & 0xff) as u8,
        (type_ & 0xff) as u8,
    ])
}

/// An enum used to track the type of box so that the correct values can be
/// read from the header.
#[derive(Clone, PartialEq)]
pub enum BoxType {
    BasicBox,
    FullBox,
}

/// Parsed mp4 box.
pub struct ParsedBox {
    /// The box name, a 4-character string (fourcc).
    pub name: String,
    /// The parser that parsed this box. The parser can be used to parse child
    /// boxes where the configuration of the current parser is needed to parsed
    /// other boxes.
    pub parser: Mp4Parser,
    /// If true, allows reading partial payloads from some boxes. If the goal is a
    /// child box, we can sometimes find it without enough data to find all child
    /// boxes. This property allows the partialOkay flag from parse() to be
    /// propagated through methods like children().
    pub partial_okay: bool,
    /// If true, stop reading if an incomplete box is detected.
    pub stop_on_partial: bool,
    /// The start of this box (before the header) in the original buffer. This
    /// start position is the absolute position.
    pub start: u64, // i64
    /// The size of this box (including the header).
    pub size: usize,
    /// The version for a full box, null for basic boxes.
    pub version: Option<u32>,
    /// The flags for a full box, null for basic boxes.
    pub flags: Option<u32>,
    /// The reader for this box is only for this box. Reading or not reading to
    /// the end will have no affect on the parser reading other sibling boxes.
    pub reader: Reader,
    /// If true, the box header had a 64-bit size field.  This affects the offsets
    /// of other fields.
    pub has_64_bit_size: bool,
}

impl ParsedBox {
    /// Find the header size of the box.
    /// Useful for modifying boxes in place or finding the exact offset of a field.
    pub fn header_size(&self) -> u64 {
        let basic_header_size = 8;
        let _64_bit_field_size = if self.has_64_bit_size { 8 } else { 0 };
        let version_and_flags_size = if self.flags.is_some() { 4 } else { 0 };
        basic_header_size + _64_bit_field_size + version_and_flags_size
    }
}
