use super::Reader;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

const BASIC_BOX: u8 = 0;
const FULL_BOX: u8 = 1;

pub(super) type DataHandler = Arc<dyn Fn(Vec<u8>)>;
pub(super) type BoxHandler = Arc<dyn Fn(ParsedBox)>;

#[allow(dead_code)]
pub(super) struct TFHD {
    pub(super) track_id: u32,
    pub(super) default_sample_duration: u32,
    pub(super) default_sample_size: u32,
}

pub(super) struct TRUN {
    pub(super) sample_count: u32,
    pub(super) sample_data: Vec<Sample>,
}

#[derive(Default, Clone)]
pub(super) struct Sample {
    pub(super) sample_duration: u32,
    pub(super) sample_size: u32,
    pub(super) sample_composition_time_offset: u32,
}

#[derive(Clone)]
pub(super) struct ParsedBox {
    parser: MP4Parser,
    partial_okay: bool,
    start: i64,
    pub(super) version: u32,
    pub(super) flags: u32,
    pub(super) reader: Reader,
    has_64_bit_size: bool,
}

impl Default for ParsedBox {
    fn default() -> Self {
        Self {
            parser: MP4Parser::default(),
            partial_okay: false,
            start: 0,
            version: 1000,
            flags: 1000,
            reader: Reader::default(),
            has_64_bit_size: false,
        }
    }
}

impl ParsedBox {
    fn header_size(&self) -> u8 {
        8 + if self.has_64_bit_size { 8 } else { 0 } + if self.flags != 0 { 4 } else { 0 }
    }
}

#[derive(Default, Clone)]
pub(super) struct MP4Parser {
    done: bool,
    headers: HashMap<i64, i32>,
    box_definitions: HashMap<i64, BoxHandler>,
}

impl MP4Parser {
    pub(super) fn basic(mut self, _type: &str, handler: BoxHandler) -> Self {
        let type_code = type_from_string(_type);
        self.headers.insert(type_code as i64, BASIC_BOX as i32);
        self.box_definitions.insert(type_code as i64, handler);
        self
    }

    pub(super) fn full(mut self, _type: &str, handler: BoxHandler) -> Self {
        let type_code = type_from_string(_type);
        self.headers.insert(type_code as i64, FULL_BOX as i32);
        self.box_definitions.insert(type_code as i64, handler);
        self
    }

    pub(super) fn parse(
        &mut self,
        data: &[u8],
        partial_okay: Option<bool>,
        stop_on_partial: Option<bool>,
    ) {
        let mut reader = Reader {
            cursor: Cursor::new(data.to_vec()),
        };

        self.done = false;

        while reader.has_more_data() && !self.done {
            self.parse_next(
                0,
                &mut reader,
                partial_okay.unwrap_or(false),
                stop_on_partial,
            );
        }
    }

    fn parse_next(
        &mut self,
        abs_start: i64,
        reader: &mut Reader,
        partial_okay: bool,
        stop_on_partial: Option<bool>,
    ) {
        let stop_on_partial = stop_on_partial.unwrap_or(false);
        let start = reader.get_position();

        if stop_on_partial && start + 8 > reader.get_length() {
            self.done = true;
            return ();
        }

        let mut size = reader.read_u32() as i64;
        let _type = reader.read_u32();
        // let name = type_to_string(_type);
        let mut has_64_bit_size = false;

        // println!("{}", name);

        match size {
            0 => {
                size = (reader.get_length() - start) as i64;
            }
            1 => {
                if stop_on_partial && reader.get_position() + 8 > reader.get_length() {
                    self.done = true;
                    return ();
                }
                size = reader.read_u64() as i64;
                has_64_bit_size = true;
            }
            _ => (),
        }

        let box_definition = self.box_definitions.get(&(_type as i64));

        if let Some(box_definition) = box_definition {
            let mut version = 1000_u32;
            let mut flags = 1000_u32;

            if *self.headers.get(&(_type as i64)).unwrap() == FULL_BOX as i32 {
                if stop_on_partial && reader.get_position() + 4 > reader.get_length() {
                    self.done = true;
                    return ();
                }

                let version_and_flags = reader.read_u32();
                version = version_and_flags >> 24;
                flags = version_and_flags & 0xFFFFFF;
            }

            let mut end = start + size as usize;

            if partial_okay && end > reader.get_length() {
                end = reader.get_length();
            }

            if stop_on_partial && end > reader.get_length() {
                self.done = true;
                return ();
            }

            let payload_size = end - reader.get_position();
            let payload = if payload_size > 0 {
                reader.read_bytes(payload_size)
            } else {
                Vec::with_capacity(0)
            };

            let _box = ParsedBox {
                parser: self.clone().to_owned(),
                partial_okay: partial_okay || false,
                start: start as i64 + abs_start,
                version,
                flags,
                reader: Reader {
                    cursor: Cursor::new(payload),
                },
                has_64_bit_size,
            };

            box_definition(_box);
        } else {
            reader.read_bytes(
                (start + size as usize - reader.get_position())
                    .min(reader.get_length() - reader.get_position()),
            );
        }
    }
}

pub(super) fn alldata(handler: DataHandler) -> BoxHandler {
    Arc::new(move |mut _box: ParsedBox| {
        let all = _box.reader.get_length() - _box.reader.get_position() as usize;
        handler(_box.reader.read_bytes(all))
    })
}

pub(super) fn children(mut _box: ParsedBox) {
    while _box.reader.has_more_data() && !_box.parser.done {
        _box.parser.parse_next(
            _box.start + _box.header_size() as i64,
            &mut _box.reader,
            _box.partial_okay,
            None,
        );
    }
}

pub(super) fn sample_description(mut _box: ParsedBox) {
    let header_size = _box.header_size();

    for _ in 0..(_box.reader.read_u32()) {
        _box.parser.parse_next(
            _box.start + header_size as i64,
            &mut _box.reader,
            _box.partial_okay,
            None,
        );

        if _box.parser.done {
            break;
        }
    }
}

pub(super) fn type_to_string(_type: u32) -> String {
    String::from_utf8(vec![
        ((_type >> 24) & 0xff) as u8,
        ((_type >> 16) & 0xff) as u8,
        ((_type >> 8) & 0xff) as u8,
        (_type & 0xff) as u8,
    ])
    .unwrap()
}

pub(super) fn type_from_string(name: &str) -> i32 {
    if name.len() != 4 {
        // throw new Exception("Mp4 box names must be 4 characters long");
    }

    let mut code = 0;

    for chr in name.chars() {
        code = (code << 8) | chr as i32;
    }

    code
}
