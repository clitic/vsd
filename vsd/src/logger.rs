use colored::Colorize;
use log::{Level, Metadata, Record};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_label = match record.level() {
                Level::Debug => "[DEBUG]".bold().blue(),
                Level::Error => "[ERROR]".bold().red(),
                Level::Info => "[INFO]".bold().green(),
                Level::Trace => "[TRACE]".bold().purple(),
                Level::Warn => "[WARN]".bold().yellow(),
            };

            println!("{} {}", level_label, record.args());
        }
    }

    fn flush(&self) {}
}
