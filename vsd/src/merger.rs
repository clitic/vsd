// use crate::progress::DownloadProgress;
use anyhow::Result;
use std::{
    collections::HashMap,
    fs,
    fs::File,
    io::Write,
    path::PathBuf,
};

pub(super) struct Merger {
    size: usize,
    file: File,
    pos: usize,
    buffers: HashMap<usize, Vec<u8>>,
    stored_bytes: usize,
    flushed_bytes: usize,
    indexed: usize,

    directory: Option<PathBuf>,
}

impl Merger {
    pub(super) fn new(size: usize, filename: &str) -> Result<Self> {
        Ok(Self {
            size: size - 1,
            file: File::create(filename)?,
            pos: 0,
            buffers: HashMap::new(),
            stored_bytes: 0,
            flushed_bytes: 0,
            indexed: 0,
            directory: None,
        })
    }

    pub(super) fn with_directory(size: usize, directory: &str) -> Result<Self> {
        let directory = PathBuf::from(directory);

        if !directory.exists() {
            fs::create_dir_all(&directory)?;
        }

        Ok(Self {
            size: size - 1,
            file: File::create(directory.join(format!(
                "0.{}",
                directory.extension().unwrap().to_string_lossy()
            )))?,
            pos: 0,
            buffers: HashMap::new(),
            stored_bytes: 0,
            flushed_bytes: 0,
            indexed: 0,
            directory: Some(directory),
        })
    }

    pub(super) fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        if let Some(directory) = &self.directory {
            self.file = File::create(directory.join(format!(
                "{}.{}",
                pos,
                directory.extension().unwrap().to_string_lossy()
            )))?;
        }

        if self.directory.is_some() || (pos == 0 || (self.pos != 0 && self.pos == pos)) {
            self.file.write_all(buf)?;
            self.file.flush()?;
            self.pos += 1;
            let size = buf.len();
            self.stored_bytes += size;
            self.flushed_bytes += size;
        } else {
            self.buffers.insert(pos, buf.to_vec());
            self.stored_bytes += buf.len();
        }

        self.indexed += 1;
        Ok(())
    }

    pub(super) fn flush(&mut self) -> Result<()> {
        while self.pos <= self.size {
            let op_buf = self.buffers.remove(&self.pos);

            if let Some(buf) = op_buf {
                self.file.write_all(&buf)?;
                self.file.flush()?;
                self.pos += 1;
                self.flushed_bytes += buf.len();
                // self.update()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    // pub(super) fn position(&self) -> usize {
    //     self.pos
    // }

    pub(super) fn buffered(&self) -> bool {
        self.buffers.is_empty() && self.pos >= (self.size + 1)
    }

    pub(super) fn stored(&self) -> usize {
        self.stored_bytes
    }

    pub(super) fn estimate(&self) -> usize {
        if self.indexed == 0 {
            0
        } else {
            (self.stored_bytes / self.indexed) * (self.size + 1)
        }
    }
}
