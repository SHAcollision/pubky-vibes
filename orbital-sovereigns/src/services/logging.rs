use dioxus::prelude::{Signal, WritableExt};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogLevel {
    pub fn css_class(self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Success => "success",
            LogLevel::Warning => "warn",
            LogLevel::Error => "error",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: OffsetDateTime,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

pub fn push_log(mut logs: Signal<Vec<LogEntry>>, level: LogLevel, message: impl Into<String>) {
    let entry = LogEntry::new(level, message);
    logs.write().push(entry);
}
