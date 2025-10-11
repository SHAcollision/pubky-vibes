use dioxus::prelude::{Signal, WritableExt};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Error,
}

#[derive(Clone)]
pub struct LogEntry {
    level: LogLevel,
    message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }

    pub fn class(&self) -> &'static str {
        match self.level {
            LogLevel::Info => "log-info",
            LogLevel::Success => "log-success",
            LogLevel::Error => "log-error",
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub fn push_log(mut logs: Signal<Vec<LogEntry>>, level: LogLevel, message: impl Into<String>) {
    let mut entries = logs.write();
    entries.push(LogEntry::new(level, message));
    if entries.len() > 200 {
        let drop = entries.len() - 200;
        entries.drain(0..drop);
    }
}
