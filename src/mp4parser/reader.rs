/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/f539147d480fff9cc8d685f3aac0e6f5dc28a182/lib/util/data_view_reader.js

*/

use std::io::{Cursor, Error, ErrorKind, Read, Result};

#[derive(Clone, Default)]
pub(super) struct Reader {
    inner: Cursor<Vec<u8>>,
    little_endian: bool,
}

impl Reader {
    pub(super) fn new(data: &[u8], little_endian: bool) -> Self {
        Self {
            inner: Cursor::new(data.to_vec()),
            little_endian,
        }
    }

    pub(super) fn has_more_data(&self) -> bool {
        self.inner.position() < (self.inner.get_ref().len() as u64)
    }

    pub(super) fn get_length(&self) -> u64 {
        self.inner.get_ref().len() as u64
    }

    pub(super) fn get_position(&self) -> u64 {
        self.inner.position()
    }

    pub(super) fn set_position(&self, pos: u64) {
        self.inner.set_position(pos)
    }

    pub(super) fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u16::from_le_bytes(buf))
        } else {
            Ok(u16::from_be_bytes(buf))
        }
    }

    pub(super) fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(i32::from_le_bytes(buf))
        } else {
            Ok(i32::from_be_bytes(buf))
        }
    }

    pub(super) fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u32::from_le_bytes(buf))
        } else {
            Ok(u32::from_be_bytes(buf))
        }
    }

    pub(super) fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0; 8];
        self.inner.read_exact(&mut buf)?;

        if self.little_endian {
            Ok(u64::from_le_bytes(buf))
        } else {
            Ok(u64::from_be_bytes(buf))
        }
    }

    pub(super) fn read_bytes(&mut self, bytes: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; bytes];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub(super) fn skip(&mut self, bytes: u64) -> Result<()> {
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
