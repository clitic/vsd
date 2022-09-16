mod capture;
mod collect;
mod decrypt;
mod extract;
mod merge;
mod save;
mod types;

pub use capture::Capture;
pub use collect::Collect;
pub use decrypt::Decrypt;
pub use extract::Extract;
pub use merge::Merge;
pub use save::Save;
pub use types::{InputType, Quality};

use clap::{Parser, Subcommand};

/// Download adaptive live streams from websites, HLS and Dash playlists.
///
/// Know more about adaptive live streams from https://howvideo.works
#[derive(Debug, Clone, Parser)]
#[clap(version, author = "clitic <clitic21@gmail.com>", about)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    Capture(Capture),
    Collect(Collect),
    Decrypt(Decrypt),
    Extract(Extract),
    Merge(Merge),
    Save(Save),
    // Check(Check),
}
