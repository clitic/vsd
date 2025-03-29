use anyhow::Result;
use std::{collections::HashMap, fs, io::Write, path::PathBuf};

enum MergerType {
    Directory(PathBuf),
    File((fs::File, HashMap<usize, Vec<u8>>)),
}

pub struct Merger {
    indexed: usize,
    merger_type: MergerType,
    pos: usize,
    size: usize,
    stored_bytes: usize,
}

impl Merger {
    pub fn new_file(size: usize, path: &PathBuf) -> Result<Self> {
        Ok(Self {
            indexed: 0,
            merger_type: MergerType::File((fs::File::create(path)?, HashMap::new())),
            pos: 0,
            size: size - 1,
            stored_bytes: 0,
        })
    }

    pub fn new_directory(size: usize, path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        Ok(Self {
            indexed: 0,
            merger_type: MergerType::Directory(path.to_owned()),
            pos: 0,
            stored_bytes: 0,
            size: size - 1,
        })
    }

    pub fn write(&mut self, pos: usize, buf: &[u8]) -> Result<()> {
        match &mut self.merger_type {
            MergerType::Directory(path) => {
                let mut file = fs::File::create(path.join(format!(
                    "{}.{}",
                    pos,
                    path.extension().unwrap().to_string_lossy()
                )))?;
                file.write_all(buf)?;
                file.flush()?;
                self.pos += 1;
                let size = buf.len();
                self.stored_bytes += size;
            }
            MergerType::File((file, buffers)) => {
                if pos == 0 || (self.pos != 0 && self.pos == pos) {
                    file.write_all(buf)?;
                    file.flush()?;
                    self.pos += 1;
                    let size = buf.len();
                    self.stored_bytes += size;
                } else {
                    buffers.insert(pos, buf.to_vec());
                    self.stored_bytes += buf.len();
                }
            }
        };

        self.indexed += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if let MergerType::File((file, buffers)) = &mut self.merger_type {
            while self.pos <= self.size {
                if let Some(buf) = buffers.remove(&self.pos) {
                    file.write_all(&buf)?;
                    file.flush()?;
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn buffered(&self) -> bool {
        let buffers_empty = match &self.merger_type {
            MergerType::Directory(_) => true,
            MergerType::File((_, buffers)) => buffers.is_empty(),
        };
        buffers_empty && self.pos >= (self.size + 1)
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
