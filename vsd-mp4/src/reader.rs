/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/f539147d480fff9cc8d685f3aac0e6f5dc28a182/lib/util/data_view_reader.js

*/

use std::io::{Cursor, Error, ErrorKind, Read, Result};

/// Reader for parsing mp4 files.
#[derive(Clone, Default)]
pub struct Reader {
    inner: Cursor<Vec<u8>>,
    little_endian: bool,
}

impl Reader {
    pub fn new(data: &[u8], little_endian: bool) -> Self {
        Self {
            inner: Cursor::new(data.to_vec()),
            little_endian,
        }
    }

    pub fn has_more_data(&self) -> bool {
        self.inner.position() < (self.inner.get_ref().len() as u64)
    }

    pub fn get_length(&self) -> u64 {
        self.inner.get_ref().len() as u64
    }

    pub fn get_position(&self) -> u64 {
        self.inner.position()
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u16::from_le_bytes(buf))
        } else {
            Ok(u16::from_be_bytes(buf))
        }
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(i32::from_le_bytes(buf))
        } else {
            Ok(i32::from_be_bytes(buf))
        }
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u32::from_le_bytes(buf))
        } else {
            Ok(u32::from_be_bytes(buf))
        }
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0; 8];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u64::from_le_bytes(buf))
        } else {
            Ok(u64::from_be_bytes(buf))
        }
    }

    pub fn read_bytes_u8(&mut self, bytes: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; bytes];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    // https://stackoverflow.com/questions/73176253/how-to-reencode-a-utf-16-byte-array-as-utf-8
    pub fn read_bytes_u16(&mut self, bytes: usize) -> Result<Vec<u16>> {
        Ok(self
            .read_bytes_u8(bytes)?
            .chunks(2)
            .map(|x| {
                if self.little_endian {
                    u16::from_le_bytes(x.try_into().unwrap())
                } else {
                    u16::from_be_bytes(x.try_into().unwrap())
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn skip(&mut self, bytes: u64) -> Result<()> {
        let position = self.get_position() + bytes;

        if position > self.get_length() {
            return Err(Error::new(
                ErrorKind::OutOfMemory,
                "mp4reader: out of bounds",
            ));
        }

        self.inner.set_position(position);
        Ok(())
    }
}
