mod extract;
mod merge;
mod save;

#[cfg(feature = "browser")]
mod capture;

pub use extract::Extract;
pub use merge::Merge;
pub use save::Save;

#[cfg(feature = "browser")]
pub use capture::Capture;

use clap::{ColorChoice, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    about,
    author = "clitic <clitic21@gmail.com>",
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\n\nEnabled features:",
        "\n  browser                 : ", cfg!(feature = "browser"),
        "\n  native-tls              : ", cfg!(feature = "native-tls"),
        "\n  rustls-tls-native-roots : ", cfg!(feature = "rustls-tls-native-roots"),
        "\n  rustls-tls-webpki-roots : ", cfg!(feature = "rustls-tls-webpki-roots"),
    ),
    version,
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// When to output colored text.
    #[arg(long, global = true, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[cfg(feature = "browser")]
    Capture(Capture),
    Extract(Extract),
    Merge(Merge),
    Save(Box<Save>),
}
