// use crate::progress::DownloadProgress;
use anyhow::Result;
use std::{collections::HashMap, fs::File, io::Write};

pub(super) struct Merger {
    size: usize,
    file: File,
    pos: usize,
    buffers: HashMap<usize, Vec<u8>>,
    stored_bytes: usize,
    flushed_bytes: usize,
    indexed: usize,
    // progress: DownloadProgress,
    // json_file: File,
}

impl Merger {
    // pub(super) fn new(size: usize, filename: &str, progress: DownloadProgress) -> Result<Self> {
    pub(super) fn new(size: usize, filename: &str) -> Result<Self> {
        Ok(Self {
            size: size - 1,
            file: File::create(filename)?,
            pos: 0,
            buffers: HashMap::new(),
            stored_bytes: 0,
            flushed_bytes: 0,
            indexed: 0,
            // json_file: File::create(&progress.file)?,
            // progress,
        })
    }

    // pub(super) fn try_from_json(size: usize, filename: &str, json_file: String) -> Result<Self> {
    //     if !Path::new(&json_file).exists() {
    //         bail!("Can't resume because {} doesn't exists.", json_file)
    //     }

    //     let progress: DownloadProgress = serde_json::from_reader(std::fs::File::open(&json_file)?)?;
    //     let mut pos = progress.downloaded("video");

    //     let file = if Path::new(filename).exists() {
    //         std::fs::OpenOptions::new().append(true).open(filename)?
    //     } else {
    //         pos = 0;
    //         File::create(filename)?
    //     };

    //     let stored_bytes = file.metadata()?.len() as usize;

    //     Ok(Self {
    //         size: size - 1,
    //         file,
    //         pos,
    //         buffers: HashMap::new(),
    //         stored_bytes,
    //         flushed_bytes: stored_bytes,
    //         indexed: pos,
    //         progress,
    //         json_file: std::fs::OpenOptions::new().append(true).open(&json_file)?,
    //     })
    // }

    pub(super) fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        if pos == 0 || (self.pos != 0 && self.pos == pos) {
            self.file.write_all(buf)?;
            self.file.flush()?;
            self.pos += 1;
            let size = buf.len();
            self.stored_bytes += size;
            self.flushed_bytes += size;
            // self.update()?;
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

    // pub(super) fn update(&mut self) -> Result<()> {
    //     self.progress
    //         .update("video", self.pos, &mut self.json_file)?;
    //     Ok(())
    // }
}
