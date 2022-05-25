use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;

pub struct BinarySequence {
    size: usize,
    file: std::fs::File,
    pos: usize,
    buffers: HashMap<usize, Vec<u8>>,
    stored_bytes: usize,
    flushed_bytes: usize,
	indexed: usize,
}

impl BinarySequence {
    pub fn new(size: usize, filename: String) -> Self {
        Self {
            size: size - 1,
            file: std::fs::File::create(filename).unwrap(),
            pos: 0,
            buffers: HashMap::new(),
            stored_bytes: 0,
            flushed_bytes: 0,
			indexed: 0,
        }
    }

    pub fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        if pos == 0 || (self.pos != 0 && self.pos == pos) {
            //println!("writing={}", pos);
            self.file.write(buf)?;
            self.file.flush()?;
            // self.file.sync_all()?;
            self.pos += 1;
            let size = buf.len();
            self.stored_bytes += size;
            self.flushed_bytes += size;
        } else {
            //println!("storing={}", pos);
            self.buffers.insert(pos, buf.to_vec());
            self.stored_bytes += buf.len();
        }
		
		self.indexed += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        while self.pos <= self.size {
            let op_buf = self.buffers.remove(&self.pos);

            if let Some(buf) = op_buf {
                //println!("writing_on_flush={}", self.pos);
                self.file.write(&buf)?;
                self.file.flush()?;
                // self.file.sync_all()?;
                self.pos += 1;
                self.flushed_bytes += buf.len();
            } else {
                //println!("can't get {}", self.pos);
                break;
            }
        }

        Ok(())
    }

    pub fn buffered(&self) -> bool {
        //println!("{:?} {}", self.buffers.keys(), self.pos);
        self.buffers.len() == 0
    }

    pub fn stored(&self) -> usize {
        self.stored_bytes
    }

    pub fn estimate(&self) -> usize {
        if self.indexed == 0 {
            0
        } else {
            (self.stored_bytes / self.indexed) * (self.size + 1)
        }
    }
}
