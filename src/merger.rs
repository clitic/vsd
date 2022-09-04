use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};

use anyhow::{bail, Result};

use crate::Progress;

pub struct BinaryMerger {
    size: usize,
    file: std::fs::File,
    pos: usize,
    buffers: HashMap<usize, Vec<u8>>,
    stored_bytes: usize,
    flushed_bytes: usize,
    indexed: usize,
    progress: Progress,
    json_file: std::fs::File,
}

impl BinaryMerger {
    pub fn new(size: usize, filename: String, progress: Progress) -> Result<Self> {
        let json_file = progress.json_file.clone();

        Ok(Self {
            size: size - 1,
            file: std::fs::File::create(&filename)?,
            pos: 0,
            buffers: HashMap::new(),
            stored_bytes: 0,
            flushed_bytes: 0,
            indexed: 0,
            progress,
            json_file: std::fs::File::create(json_file)?,
        })
    }

    pub fn try_from_json(size: usize, filename: String, json_file: String) -> Result<Self> {
        if !std::path::Path::new(&json_file).exists() {
            bail!("Can't resume because {} doesn't exists.", json_file)
        }

        let progress: Progress = serde_json::from_reader(std::fs::File::open(&json_file)?)?;
        let mut pos = progress.downloaded();

        let file = if std::path::Path::new(&filename).exists() {
            std::fs::OpenOptions::new().append(true).open(filename)?
        } else {
            pos = 0;
            std::fs::File::create(&filename)?
        };

        let stored_bytes = file.metadata()?.len() as usize;

        Ok(Self {
            size: size - 1,
            file,
            pos,
            buffers: HashMap::new(),
            stored_bytes,
            flushed_bytes: stored_bytes,
            indexed: pos,
            progress,
            json_file: std::fs::OpenOptions::new().append(true).open(&json_file)?,
        })
    }

    pub fn reset(&mut self, size: usize, filename: String) -> Result<()> {
        self.size = size - 1;
        self.file = std::fs::File::create(&filename)?;
        self.pos = 0;
        self.buffers = HashMap::new();
        self.stored_bytes = 0;
        self.flushed_bytes = 0;
        self.indexed = 0;
        Ok(())
    }

    pub fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        if pos == 0 || (self.pos != 0 && self.pos == pos) {
            self.file.write_all(buf)?;
            self.file.flush()?;
            self.pos += 1;
            let size = buf.len();
            self.stored_bytes += size;
            self.flushed_bytes += size;
            self.update()?;
        } else {
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
                self.file.write_all(&buf)?;
                self.file.flush()?;
                self.pos += 1;
                self.flushed_bytes += buf.len();
                self.update()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn buffered(&self) -> bool {
        self.buffers.is_empty() && self.pos >= (self.size + 1)
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

    pub fn relative_estimate(&self, size: usize) -> usize {
        if self.indexed == 0 {
            0
        } else {
            (self.stored_bytes / self.indexed) * (size + 1)
        }
    }

    pub fn update(&mut self) -> Result<()> {
        self.json_file.seek(SeekFrom::Start(0))?;
        self.progress
            .update(self.pos, self.size + 1, &self.json_file);
        Ok(())
    }
}

pub struct Estimater {
    stored_bytes: usize,
}

impl Estimater {
    pub fn stored(&self) -> usize {
        self.stored_bytes
    }

    pub fn estimate(&self, indexed: usize, size: usize) -> usize {
        if indexed == 0 {
            0
        } else {
            (self.stored_bytes / indexed) * (size + 1)
        }
    }
}
