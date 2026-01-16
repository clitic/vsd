/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/f539147d480fff9cc8d685f3aac0e6f5dc28a182/lib/util/data_view_reader.js

*/

use std::io::{Cursor, Error, ErrorKind, Read, Result};

#[derive(Clone, Default)]
enum Endianness {
    #[default]
    Big,
    Little,
}

/// Reader for parsing mp4 files.
#[derive(Clone, Default)]
pub struct Reader {
    endian: Endianness,
    inner: Cursor<Vec<u8>>,
}

impl Reader {
    pub fn new_big_endian(data: Vec<u8>) -> Self {
        Self {
            endian: Endianness::Big,
            inner: Cursor::new(data),
        }
    }

    pub fn new_little_endian(data: Vec<u8>) -> Self {
        Self {
            endian: Endianness::Little,
            inner: Cursor::new(data),
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

    pub fn skip(&mut self, bytes: u64) -> Result<()> {
        let position = self.get_position() + bytes;

        if position > self.get_length() {
            return Err(Error::new(
                ErrorKind::OutOfMemory,
                "Reader skips out of memory bounds.",
            ));
        }

        self.inner.set_position(position);
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.inner.read_exact(&mut buf)?;

        match self.endian {
            Endianness::Big => Ok(u8::from_be_bytes(buf)),
            Endianness::Little => Ok(u8::from_le_bytes(buf)),
        }
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.inner.read_exact(&mut buf)?;

        match self.endian {
            Endianness::Big => Ok(u16::from_be_bytes(buf)),
            Endianness::Little => Ok(u16::from_le_bytes(buf)),
        }
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        match self.endian {
            Endianness::Big => Ok(u32::from_be_bytes(buf)),
            Endianness::Little => Ok(u32::from_le_bytes(buf)),
        }
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0; 8];
        self.inner.read_exact(&mut buf)?;

        match self.endian {
            Endianness::Big => Ok(u64::from_be_bytes(buf)),
            Endianness::Little => Ok(u64::from_le_bytes(buf)),
        }
    }

    pub fn read_bytes_u8(&mut self, bytes: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; bytes];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_bytes_u16(&mut self, bytes: usize) -> Result<Vec<u16>> {
        Ok(self
            .read_bytes_u8(bytes)?
            .chunks_exact(2)
            .map(|x| match self.endian {
                Endianness::Big => u16::from_be_bytes([x[0], x[1]]),
                Endianness::Little => u16::from_le_bytes([x[0], x[1]]),
            })
            .collect())
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;

        match self.endian {
            Endianness::Big => Ok(i32::from_be_bytes(buf)),
            Endianness::Little => Ok(i32::from_le_bytes(buf)),
        }
    }
}
