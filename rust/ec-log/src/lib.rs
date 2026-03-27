use std::fs;
use std::path::{Path, PathBuf};

use tracing::Level;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!(
                "unknown log level '{value}'; expected error, warn, info, debug, or trace"
            )),
        }
    }

    fn as_level(self) -> Level {
        match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
            Self::Trace => Level::TRACE,
        }
    }
}

pub fn init_file_logging(path: &Path, level: LogLevel) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let path = PathBuf::from(path);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(move || {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("log file should be openable after initialization")
        })
        .with_filter(LevelFilter::from_level(level.as_level()));
    tracing_subscriber::registry().with(fmt_layer).try_init()?;
    Ok(())
}
