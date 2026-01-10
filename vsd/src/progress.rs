use colored::Colorize;
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Instant,
};

struct ProgressInner {
    counter: usize,
    id: String,
    total: usize,
    last_bytes: usize,
    last_time: Instant,
    total_bytes: usize,
}

#[derive(Clone)]
pub struct Progress {
    inner: Arc<Mutex<ProgressInner>>,
}

impl Progress {
    pub fn new(id: &str, total: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressInner {
                counter: 0,
                id: id.to_owned(),
                total,
                last_bytes: 0,
                last_time: Instant::now(),
                total_bytes: 0,
            })),
        }
    }

    pub fn update(&self, chunk_bytes: usize) {
        let mut inner = self.inner.lock().unwrap();

        inner.counter += 1;
        inner.total_bytes += chunk_bytes;

        let now = Instant::now();
        let elapsed_secs = now.duration_since(inner.last_time).as_secs_f64();
        let remaining_bytes =
            ((inner.total_bytes as f64 / inner.counter as f64) * inner.total as f64) as usize;
        let percent = if inner.total > 0 {
            (inner.counter as f64 / inner.total as f64 * 100.0) as usize
        } else {
            100
        };

        // FIX - Speed and ETA smoothning
        let speed = if elapsed_secs > 0.0 {
            (inner.total_bytes.saturating_sub(inner.last_bytes)) as f64 / elapsed_secs
        } else {
            0.0
        };
        let eta_secs = (inner.total.saturating_sub(inner.counter) as f64 * elapsed_secs) as usize;
        // let eta_secs = (remaining_bytes as f64 / speed) as usize;

        let stderr = io::stderr();
        let mut handle = stderr.lock();
        write!(
            handle,
            "\r\x1B[2K{}#[{}] {}{} PT:{} DL:{} ETA:{}{}",
            "[".magenta(),
            inner.id,
            format!(
                "{}/~{}",
                ByteSize(inner.total_bytes),
                ByteSize(remaining_bytes)
            ),
            format!("({}%)", percent).cyan(),
            format!("{}/{}", inner.counter, inner.total).cyan(),
            ByteSize(speed as usize).to_string().green(),
            Eta(eta_secs).to_string().yellow(),
            "]".magenta(),
        )
        .unwrap();
        handle.flush().unwrap();

        inner.last_bytes = inner.total_bytes;
        inner.last_time = now;
    }
}

pub struct ByteSize(pub usize);

impl std::fmt::Display for ByteSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const KIB: f64 = 1024.0;
        const MIB: f64 = KIB * 1024.0;
        const GIB: f64 = MIB * 1024.0;

        let bytes = self.0 as f64;

        if bytes >= GIB {
            write!(f, "{:.1}GiB", bytes / GIB)
        } else if bytes >= MIB {
            write!(f, "{:.1}MiB", bytes / MIB)
        } else if bytes >= KIB {
            write!(f, "{:.1}KiB", bytes / KIB)
        } else {
            write!(f, "{}B", self.0)
        }
    }
}

struct Eta(usize);

impl std::fmt::Display for Eta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_seconds = self.0;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            write!(f, "{}h{}m{}s", hours, minutes, seconds)
        } else if minutes > 0 {
            write!(f, "{}m{}s", minutes, seconds)
        } else {
            write!(f, "{}s", seconds)
        }
    }
}
