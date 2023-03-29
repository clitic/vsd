mod locator;
mod parser;
mod template;
mod playlist;

use locator::DashUrl;
use template::TemplateResolver;
use parser::{iso8601_duration_to_seconds, mpd_range_to_byte_range};

pub use parser::{parse, MPD};
pub(crate) use playlist::{parse_as_master, push_segments};
