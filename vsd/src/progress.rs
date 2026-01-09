// use kdam::{Bar, BarExt, tqdm};
// use std::{io::Result, num::NonZeroU16};

// #[derive(BarExt)]
// struct Progress {
//     #[bar]
//     pb: Bar,
//     downloaded_bytes: usize,
// }

// impl Progress {
//     fn estimate(&self) -> usize {
//         
//     }
    
//     fn render(&mut self) -> String {
//         let fmt_percentage = self.pb.fmt_percentage(0);
//         let padding = 1 + fmt_percentage.chars().count() as u16 + self.pb.animation.spaces() as u16;

//         let ncols = self.pb.ncols_for_animation(padding);

//         if ncols == 0 {
//             self.pb.bar_length = padding - 1;
//             fmt_percentage
//         } else {
//             self.pb.bar_length = padding + ncols;
//             self.pb.animation.fmt_render(
//                 NonZeroU16::new(ncols).unwrap(),
//                 self.pb.percentage(),
//                 &None,
//             ) + " "
//                 + &fmt_percentage
//         }
//     }
// }


use colored::Colorize;
use std::io::{self, Write};
use std::time::{Instant};

pub struct Progress {
    gid: String,
    total_size: usize,
    last_stat_time: Instant,
    last_stat_bytes: usize,
}

impl Progress {
    pub fn new(gid: &str, total_size: usize) -> Self {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        write!(handle, "\x1B[?25l").unwrap();
        handle.flush().unwrap();

        Self {
            gid: gid.to_owned(),
            total_size,
            last_stat_time: Instant::now(),
            last_stat_bytes: 0,
        }
    }

    pub fn update(&mut self, current_bytes: usize) {
        // estimate
        // (self.downloaded_bytes / self.pb.counter) * (self.pb.total + 1)
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_stat_time).as_secs_f64();

        let speed = if elapsed_secs > 0.0 {
            (current_bytes.saturating_sub(self.last_stat_bytes)) as f64 / elapsed_secs
        } else {
            0.0
        };

        let remaining_bytes = self.total_size.saturating_sub(current_bytes);

        let eta_seconds = if speed > 0.0 {
            (remaining_bytes as f64 / speed) as usize
        } else {
            0
        };

        let percent = if self.total_size > 0 {
            (current_bytes as f64 / self.total_size as f64 * 100.0) as usize
        } else {
            100
        };

        let progress_str = format!("{}/{}", ByteSize(current_bytes), ByteSize(self.total_size),);
        let speed_val = ByteSize(speed as usize).to_string();
        let eta_val = Eta(eta_seconds).to_string();

        let stderr = io::stderr();
        let mut handle = stderr.lock();
        write!(
            handle,
            "\r\x1B[2K{}#[{}] {}{} SG:{} DL:{} ETA:{}{}", // \x1B[2K clears the line
            "[".magenta(),
            self.gid,
            progress_str,
            format!("({}%)", percent).cyan(),
            "50/100".cyan(),
            speed_val.green(),
            eta_val.yellow(),
            "]".magenta(),
        )
        .unwrap();
        handle.flush().unwrap();

        // 4. Update state for next delta calculation
        self.last_stat_time = now;
        self.last_stat_bytes = current_bytes;
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(handle, "\x1B[?25h").unwrap();
        handle.flush().unwrap();
    }
}

struct ByteSize(usize);

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
