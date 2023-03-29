/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/d6001097a9751bd9211eb52f940e282ead026a32/lib/util/mp4_parser.js
    2. https://github.com/shaka-project/shaka-player/blob/d6001097a9751bd9211eb52f940e282ead026a32/externs/shaka/mp4_parser.js

*/

use super::Reader;
use std::collections::HashMap;
use std::sync::Arc;

type HandlerResult = Result<(), String>;
type CallbackType = Arc<dyn Fn(ParsedBox) -> HandlerResult>;

#[derive(Clone, Default)]
pub(super) struct Mp4Parser {
    headers: HashMap<usize, BoxType>,
    box_definitions: HashMap<usize, CallbackType>,
    done: bool,
}

impl Mp4Parser {
    /// Declare a box type as a Box.
    pub(super) fn _box(mut self, _type: &str, definition: CallbackType) -> Self {
        let type_code = type_from_string(_type);
        self.headers.insert(type_code, BoxType::BasicBox);
        self.box_definitions.insert(type_code, definition);
        self
    }

    /// Declare a box type as a Full Box.
    pub(super) fn full_box(mut self, _type: &str, definition: CallbackType) -> Self {
        let type_code = type_from_string(_type);
        self.headers.insert(type_code, BoxType::FullBox);
        self.box_definitions.insert(type_code, definition);
        self
    }

    /// Stop parsing. Useful for extracting information from partial segments and
    /// avoiding an out-of-bounds error once you find what you are looking for.
    pub(super) fn stop(&mut self) {
        self.done = true;
    }

    /// Parse the given data using the added callbacks.
    ///
    /// # Arguments
    ///
    /// - `partial_okay` (optional) - If true, allow reading partial payloads
    /// from some boxes. If the goal is a child box, we can sometimes find it
    /// without enough data to find all child boxes.
    /// - `stop_on_partial` (optional) - If true, stop reading if an incomplete
    /// box is detected.
    pub(super) fn parse(
        &mut self,
        data: &[u8],
        partial_okay: Option<bool>,
        stop_on_partial: Option<bool>,
    ) -> HandlerResult {
        let mut reader = Reader::new(data, false);

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
    /// byte array.
    /// - `partial_okay` (optional) - If true, allow reading partial payloads
    /// from some boxes. If the goal is a child box, we can sometimes find it
    /// without enough data to find all child boxes.
    /// - `stop_on_partial` (optional) - If true, stop reading if an incomplete
    /// box is detected.
    fn parse_next(
        &mut self,
        abs_start: u64,
        reader: &mut Reader,
        partial_okay: Option<bool>,
        stop_on_partial: Option<bool>,
    ) -> HandlerResult {
        let partial_okay = partial_okay.unwrap_or(false);
        let stop_on_partial = stop_on_partial.unwrap_or(false);
        let start = reader.get_position();

        // size(4 bytes) + type(4 bytes) = 8 bytes
        if stop_on_partial && start + 8 > reader.get_length() {
            self.done = true;
            return Ok(());
        }

        let mut size = reader
            .read_u32()
            .map_err(|_| "mp4parser: cannot read box size (u32).".to_owned())?
            as u64;
        let _type = reader
            .read_u32()
            .map_err(|_| "mp4parser: cannot read box type (u32).".to_owned())?
            as usize;
        let name = type_to_string(_type)
            .map_err(|_| format!("mp4parser: cannot convert {} (u32) to string.", _type))?;
        let mut has_64_bit_size = false;
        // println!("Parsing MP4 box {}", name);

        match size {
            0 => size = reader.get_length() - start,
            1 => {
                if stop_on_partial && reader.get_position() + 8 > reader.get_length() {
                    self.done = true;
                    return Ok(());
                }
                size = reader
                    .read_u64()
                    .map_err(|_| "mp4parser: cannot read box size (u64).".to_owned())?;
                has_64_bit_size = true;
            }
            _ => (),
        }

        let box_definition = self.box_definitions.get(&_type);

        if let Some(box_definition) = box_definition {
            let mut version = None;
            let mut flags = None;

            if *self.headers.get(&_type).unwrap() == BoxType::FullBox {
                if stop_on_partial && reader.get_position() + 4 > reader.get_length() {
                    self.done = true;
                    return Ok(());
                }

                let version_and_flags = reader.read_u32().map_err(|_| {
                    "mp4parser: cannot read box version and flags (u32).".to_owned()
                })?;
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
                reader.read_bytes_u8(payload_size as usize).map_err(|_| {
                    format!(
                        "mp4parser: cannot read box payload ({} bytes).",
                        payload_size
                    )
                })?
            } else {
                Vec::with_capacity(0)
            };

            let payload_reader = Reader::new(&payload, false);

            let _box = ParsedBox {
                name,
                parser: self.clone(),
                partial_okay,
                version,
                flags,
                reader: payload_reader,
                size: size as usize,
                start: start + abs_start,
                has_64_bit_size,
            };

            box_definition(_box)?;
        } else {
            // Move the read head to be at the end of the box.
            // If the box is longer than the remaining parts of the file, e.g. the
            // mp4 is improperly formatted, or this was a partial range request that
            // ended in the middle of a box, just skip to the end.
            let skip_length = (start + size - reader.get_position())
                .min(reader.get_length() - reader.get_position());
            reader
                .skip(skip_length)
                .map_err(|_| format!("mp4parser: cannot skip {} bytes.", skip_length))?;
        }

        Ok(())
    }
}

// CALLBACKS

/// A callback that tells the Mp4 parser to treat the body of a box as a series
/// of boxes. The number of boxes is limited by the size of the parent box.
pub(super) fn children(mut _box: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = _box.header_size();

    while _box.reader.has_more_data() && !_box.parser.done {
        _box.parser.parse_next(
            _box.start + header_size,
            &mut _box.reader,
            Some(_box.partial_okay),
            None,
        )?;
    }

    Ok(())
}

/// A callback that tells the Mp4 parser to treat the body of a box as a sample
/// description. A sample description box has a fixed number of children. The
/// number of children is represented by a 4 byte unsigned integer. Each child
/// is a box.
pub(super) fn sample_description(mut _box: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = _box.header_size();
    let count = _box
        .reader
        .read_u32()
        .map_err(|_| "mp4reader: cannot read u32.".to_owned())?;

    for _ in 0..count {
        _box.parser.parse_next(
            _box.start + header_size,
            &mut _box.reader,
            Some(_box.partial_okay),
            None,
        )?;

        if _box.parser.done {
            break;
        }
    }

    Ok(())
}

/// A callback that tells the Mp4 parser to treat the body of a box as a visual
/// sample entry.  A visual sample entry has some fixed-sized fields
/// describing the video codec parameters, followed by an arbitrary number of
/// appended children.  Each child is a box.
#[allow(dead_code)]
pub(super) fn visual_sample_entry(mut _box: ParsedBox) -> HandlerResult {
    // The "reader" starts at the payload, so we need to add the header to the
    // start position.  The header size varies.
    let header_size = _box.header_size();

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
    _box.reader
        .skip(78)
        .map_err(|_| "mp4reader: cannot skip 78 bytes.".to_owned())?;

    while _box.reader.has_more_data() && !_box.parser.done {
        _box.parser.parse_next(
            _box.start + header_size,
            &mut _box.reader,
            Some(_box.partial_okay),
            None,
        )?;
    }

    Ok(())
}

/// Create a callback that tells the Mp4 parser to treat the body of a box as a
/// binary blob and to parse the body's contents using the provided callback.
pub(super) fn alldata(callback: Arc<dyn Fn(Vec<u8>) -> HandlerResult>) -> CallbackType {
    Arc::new(move |mut _box| {
        let all = _box.reader.get_length() - _box.reader.get_position();
        callback(
            _box.reader
                .read_bytes_u8(all as usize)
                .map_err(|_| format!("mp4reader: cannot read {} bytes.", all))?,
        )
    })
}

// UTILS

/// Convert an ascii string name to the integer type for a box.
/// The name must be four characters long.
fn type_from_string(name: &str) -> usize {
    assert!(name.len() == 4, "MP4 box names must be 4 characters long");

    let mut code = 0;

    for chr in name.chars() {
        code = (code << 8) | chr as usize;
    }

    code
}

/// Convert an integer type from a box into an ascii string name.
/// Useful for debugging.
pub(super) fn type_to_string(_type: usize) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(vec![
        ((_type >> 24) & 0xff) as u8,
        ((_type >> 16) & 0xff) as u8,
        ((_type >> 8) & 0xff) as u8,
        (_type & 0xff) as u8,
    ])
}

/// An enum used to track the type of box so that the correct values can be
/// read from the header.
#[derive(Clone, PartialEq)]
enum BoxType {
    BasicBox,
    FullBox,
}

#[allow(dead_code)]
#[derive(Clone, Default)]
pub(super) struct ParsedBox {
    /// The box name, a 4-character string (fourcc).
    name: String,
    /// The parser that parsed this box. The parser can be used to parse child
    /// boxes where the configuration of the current parser is needed to parsed
    /// other boxes
    pub(super) parser: Mp4Parser,
    /// If true, allows reading partial payloads from some boxes. If the goal is a
    /// child box, we can sometimes find it without enough data to find all child
    /// boxes. This property allows the partialOkay flag from parse() to be
    /// propagated through methods like children().
    partial_okay: bool,
    /// The size of this box (including the header).
    start: u64, // i64
    /// The size of this box (including the header).
    pub(super) size: usize,
    /// The version for a full box, null for basic boxes.
    pub(super) version: Option<u32>,
    /// The flags for a full box, null for basic boxes.
    pub(super) flags: Option<u32>,
    /// The reader for this box is only for this box. Reading or not reading to
    /// the end will have no affect on the parser reading other sibling boxes.
    pub(super) reader: Reader,
    /// If true, the box header had a 64-bit size field.  This affects the offsets
    /// of other fields.
    has_64_bit_size: bool,
}

impl ParsedBox {
    /// Find the header size of the box.
    /// Useful for modifying boxes in place or finding the exact offset of a field.
    fn header_size(&self) -> u64 {
        let basic_header_size = 8;
        let _64_bit_field_size = if self.has_64_bit_size { 8 } else { 0 };
        let version_and_flags_size = if self.flags.is_some() { 4 } else { 0 };
        basic_header_size + _64_bit_field_size + version_and_flags_size
    }
}
