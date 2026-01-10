mod extract;
mod merge;
mod save;

#[cfg(feature = "browser")]
mod capture;

use anyhow::Ok;
pub use extract::Extract;
use log::LevelFilter;
pub use merge::Merge;
pub use save::Save;

#[cfg(feature = "browser")]
pub use capture::Capture;

use crate::logger::Logger;
use clap::{ArgAction, ColorChoice, Parser, Subcommand};

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[cfg(feature = "browser")]
    Capture(Capture),
    Extract(Extract),
    Merge(Merge),
    Save(Box<Save>),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about,
    author = "clitic <clitic21@gmail.com>",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\n\nEnabled features:",
        "\n  browser    : ", cfg!(feature = "browser"),
        "\n  native-tls : ", cfg!(feature = "native-tls"),
        "\n  rustls     : ", cfg!(feature = "rustls"),
    ),
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// When to output colored text.
    #[arg(long, global = true, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Silence all output and only log errors.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// Increase verbosity (-v [debug], -vv [trace]). Default logging level is set to info.
    #[arg(short, long, action = ArgAction::Count)]
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
            #[cfg(feature = "browser")]
            Commands::Capture(args) => args.execute().await?,
            Commands::Extract(args) => args.execute()?,
            Commands::Merge(args) => args.execute()?,
            Commands::Save(args) => args.execute().await?,
        }

        Ok(())
    }
}
