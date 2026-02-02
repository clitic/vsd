mod extract;
mod merge;
mod save;

#[cfg(feature = "capture")]
mod capture;

#[cfg(feature = "license")]
mod license;

use anyhow::Ok;
pub use extract::Extract;
use log::LevelFilter;
pub use merge::Merge;
pub use save::Save;

#[cfg(feature = "capture")]
pub use capture::Capture;

#[cfg(feature = "license")]
pub use license::License;

use crate::logger::Logger;
use clap::{ArgAction, ColorChoice, Parser, Subcommand};

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[cfg(feature = "capture")]
    Capture(Capture),
    Extract(Extract),
    #[cfg(feature = "license")]
    License(License),
    Merge(Merge),
    Save(Box<Save>),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about,
    author = "clitic <clitic21@gmail.com>",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"), "\n\n",
        "Enabled Features:\n",
        "capture: ", cfg!(feature = "capture"), "\n",
        "license: ", cfg!(feature = "license"), "\n",
        "rustls-tls: ", cfg!(feature = "rustls-tls"), "\n",
        "native-tls: ", cfg!(feature = "native-tls"), "\n",
        "native-tls-vendored: ", cfg!(feature = "native-tls-vendored"),
    ),
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// When to use colored output.
    #[arg(long, global = true, help_heading = "Global Options", default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Suppress all output except errors.
    #[arg(
        short,
        long,
        global = true,
        help_heading = "Global Options",
        conflicts_with = "verbose"
    )]
    quiet: bool,

    /// Increase verbosity: `-v` (debug), `-vv` (trace).
    /// The default log level is `info`.
    #[arg(short, long, global = true, help_heading = "Global Options", action = ArgAction::Count)]
    verbose: u8,
}

impl Args {
    pub async fn execute(self) -> anyhow::Result<()> {
        let level = if self.quiet {
            LevelFilter::Error
        } else {
            match self.verbose {
                0 => LevelFilter::Info,
                1 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        };

        log::set_logger(&Logger)
            .map(|()| log::set_max_level(level))
            .expect("Failed to initialize logger.");

        match self.color {
            ColorChoice::Always => colored::control::set_override(true),
            ColorChoice::Never => colored::control::set_override(false),
            _ => (),
        }

        let mut symbols = requestty::symbols::UNICODE;
        symbols.completed = '•';
        symbols.cross = 'x';
        requestty::symbols::set(symbols);

        match self.command {
            #[cfg(feature = "capture")]
            Commands::Capture(args) => args.execute().await?,
            Commands::Extract(args) => args.execute().await?,
            #[cfg(feature = "license")]
            Commands::License(args) => args.execute().await?,
            Commands::Merge(args) => args.execute().await?,
            Commands::Save(args) => args.execute().await?,
        }

        Ok(())
    }
}
