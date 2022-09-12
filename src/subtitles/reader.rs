use std::io::{Cursor, Read};
use super::{TFHD, TRUN, Sample};

#[derive(Default, Clone)]
pub(super) struct Reader {
    pub(super) cursor: Cursor<Vec<u8>>,
}

impl Reader {
    pub(super) fn has_more_data(&self) -> bool {
        (self.cursor.position() as usize) < self.cursor.clone().into_inner().len()
    }

    pub(super) fn get_length(&self) -> usize {
        self.cursor.clone().into_inner().len()
    }

    pub(super) fn get_position(&self) -> usize {
        self.cursor.position() as usize
    }

    pub(super) fn read_bytes(&mut self, size: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size);

        for _ in 0..size {
            let mut buf2 = [0; 1];
            self.cursor.read_exact(&mut buf2).unwrap();
            buf.push(buf2[0]);
        }

        buf
    }

    pub(super) fn read_i32(&mut self) -> i32 {
        let mut buf = [0; 4];
        self.cursor.read_exact(&mut buf).unwrap();
        i32::from_be_bytes(buf)
    }

    pub(super) fn read_u32(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.cursor.read_exact(&mut buf).unwrap();
        u32::from_be_bytes(buf)
    }

    pub(super) fn read_u64(&mut self) -> u64 {
        let mut buf = [0; 8];
        self.cursor.read_exact(&mut buf).unwrap();
        u64::from_be_bytes(buf)
    }

    pub(super) fn parse_mdhd(&mut self, version: u32) -> u32 {
        if version == 1 {
            self.read_bytes(8);
            self.read_bytes(8);
        } else {
            self.read_bytes(4);
            self.read_bytes(4);
        }

        self.read_u32()
    }

    pub(super) fn parse_tfdt(&mut self, version: u32) -> u64 {
        if version == 1 {
            self.read_u64()
        } else {
            self.read_u32() as u64
        }
    }

    pub(super) fn parse_tfhd(&mut self, flags: u32) -> TFHD {
        let track_id = self.read_u32();
        let mut default_sample_duration = 0_u32;
        let mut default_sample_size = 0_u32;

        if (flags & 0x000001) != 0 {
            self.read_bytes(8);
        }

        if (flags & 0x000002) != 0 {
            self.read_bytes(4);
        }

        if (flags & 0x000008) != 0 {
            default_sample_duration = self.read_u32();
        }

        if (flags & 0x000010) != 0 {
            default_sample_size = self.read_u32();
        }

        TFHD {
            track_id,
            default_sample_duration,
            default_sample_size,
        }
    }

    pub(super) fn parse_trun(&mut self, version: u32, flags: u32) -> TRUN {
        let mut trun = TRUN {
            sample_count: self.read_u32(),
            sample_data: vec![],
        };

        if (flags & 0x000001) != 0 {
            self.read_bytes(4);
        }

        if (flags & 0x000004) != 0 {
            self.read_bytes(4);
        }

        for _ in 0..trun.sample_count {
            let mut sample = Sample::default();

            if (flags & 0x000100) != 0 {
                sample.sample_duration = self.read_u32();
            }

            if (flags & 0x000200) != 0 {
                sample.sample_size = self.read_u32();
            }

            if (flags & 0x000400) != 0 {
                self.read_bytes(4);
            }

            if (flags & 0x000800) != 0 {
                sample.sample_composition_time_offset = if version == 0 {
                    self.read_u32()
                } else {
                    self.read_i32() as u32
                };
            }

            trun.sample_data.push(sample);
        }

        trun
    }
}
