use colored::{ColoredString, Colorize};
use log::{Level, LevelFilter, Metadata, Record};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match log::max_level() {
                LevelFilter::Off => (),
                LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => {
                    match record.level() {
                        Level::Info => {
                            println!("{}", record.args());
                        }
                        _ => {
                            println!("{} {}", label(record.level()), record.args());
                        }
                    }
                }
                LevelFilter::Debug | LevelFilter::Trace => {
                    let location = match (record.file(), record.line()) {
                        (Some(file), Some(line)) => format!("[{}:{}]", file, line).dimmed(),
                        _ => "[unk]".dimmed(),
                    };

                    println!(
                        "{} {} {} {}",
                        label(record.level()),
                        record.target().dimmed(),
                        location,
                        record.args()
                    );
                }
            }
        }
    }

    fn flush(&self) {}
}

fn label(level: Level) -> ColoredString {
    match level {
        Level::Debug => "[DEBUG]".bold().blue(),
        Level::Error => "[ERROR]".bold().red(),
        Level::Info => "[INFO]".bold().green(),
        Level::Trace => "[TRACE]".bold().purple(),
        Level::Warn => "[WARN]".bold().yellow(),
    }
}
