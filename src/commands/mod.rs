mod capture;
mod collect;
mod decrypt;
mod extract;
mod merge;
mod save;
mod utils;

pub use decrypt::Decrypt;
pub use extract::Extract;
pub use merge::Merge;
pub use save::{Save, Quality};

#[cfg(feature = "chrome")]
pub use capture::Capture;
#[cfg(feature = "chrome")]
pub use collect::Collect;

use clap::{Parser, Subcommand};

/// Download video streams served over HTTP from websites, HLS and DASH playlists.
///
/// Know more about adaptive video streams served over HTTP from https://howvideo.works
#[derive(Debug, Clone, Parser)]
#[command(version, author = "clitic <clitic21@gmail.com>", about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[cfg(feature = "chrome")]
    Capture(Capture),
    #[cfg(feature = "chrome")]
    Collect(Collect),
    Decrypt(Decrypt),
    Extract(Extract),
    Merge(Merge),
    Save(Save),
    // Check(Check),
    // Convert(Convert),
}
