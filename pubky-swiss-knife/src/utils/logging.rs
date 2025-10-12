use dioxus::prelude::{Signal, WritableExt};

/// Maximum number of log entries kept in memory before older ones are trimmed.
const MAX_LOG_ENTRIES: usize = 200;

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

/// Thin wrapper around the shared activity log signal with convenience helpers for
/// recording messages.
#[derive(Clone)]
pub struct ActivityLog {
    entries: Signal<Vec<LogEntry>>,
}

impl ActivityLog {
    pub fn new(entries: Signal<Vec<LogEntry>>) -> Self {
        Self { entries }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn success(&self, message: impl Into<String>) {
        self.log(LogLevel::Success, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        push_log(self.entries, level, message);
    }
}

pub fn push_log(mut logs: Signal<Vec<LogEntry>>, level: LogLevel, message: impl Into<String>) {
    let mut entries = logs.write();
    entries.push(LogEntry::new(level, message));
    if entries.len() > MAX_LOG_ENTRIES {
        let overflow = entries.len() - MAX_LOG_ENTRIES;
        entries.drain(0..overflow);
        if overflow > 0 {
            entries.insert(
                0,
                LogEntry::new(
                    LogLevel::Info,
                    format!("Trimmed {overflow} older log entries"),
                ),
            );
            if entries.len() > MAX_LOG_ENTRIES {
                entries.truncate(MAX_LOG_ENTRIES);
            }
        }
    }
}
