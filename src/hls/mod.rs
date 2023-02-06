mod parser;
mod playlist;

pub use parser::{alternative, master};
pub use playlist::{parse_as_master, push_segments};
