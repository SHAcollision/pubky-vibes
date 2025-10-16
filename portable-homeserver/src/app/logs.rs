use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::{Result, anyhow};
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::util::SubscriberInitExt;

const DEFAULT_CAPACITY: usize = 500;

static LOG_STORE: OnceLock<LogStore> = OnceLock::new();

pub(crate) fn init_logging() -> Result<LogStore> {
    if let Some(store) = LOG_STORE.get() {
        return Ok(store.clone());
    }

    let store = LogStore::with_capacity(DEFAULT_CAPACITY);

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .unwrap_or_else(|_| EnvFilter::new("debug"));

    let fmt_layer = layer()
        .with_target(true)
        .with_level(true)
        .with_writer(|| std::io::stdout());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(LogStoreLayer {
            store: store.clone(),
        })
        .with(fmt_layer)
        .try_init()?;

    LOG_STORE
        .set(store.clone())
        .map_err(|_| anyhow!("logging has already been initialized"))?;

    Ok(store)
}

pub(crate) fn log_store() -> LogStore {
    LOG_STORE
        .get()
        .expect("logging has not been initialized")
        .clone()
}

#[derive(Clone)]
pub(crate) struct LogStore {
    inner: Arc<LogStoreInner>,
}

struct LogStoreInner {
    entries: RwLock<VecDeque<LogEntry>>,
    sender: broadcast::Sender<LogEntry>,
    capacity: usize,
    counter: AtomicU64,
}

impl LogStore {
    fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            inner: Arc::new(LogStoreInner {
                entries: RwLock::new(VecDeque::with_capacity(capacity)),
                sender,
                capacity,
                counter: AtomicU64::new(1),
            }),
        }
    }

    pub(crate) fn push(&self, mut entry: LogEntry) {
        let sequence = self.inner.counter.fetch_add(1, Ordering::Relaxed);
        entry = entry.assign_sequence(sequence);

        {
            let mut entries = self
                .inner
                .entries
                .write()
                .expect("log store mutex poisoned");
            if entries.len() >= self.inner.capacity {
                entries.pop_front();
            }
            entries.push_back(entry.clone());
        }

        let _ = self.inner.sender.send(entry);
    }

    pub(crate) fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .entries
            .read()
            .expect("log store mutex poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.inner.sender.subscribe()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.inner.capacity
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    pub(crate) sequence: u64,
    pub(crate) formatted_timestamp: String,
    pub(crate) level: Level,
    pub(crate) target: String,
    pub(crate) message: String,
    pub(crate) fields: Vec<LogField>,
}

impl LogEntry {
    fn new(level: Level, target: &str, message: String, fields: Vec<LogField>) -> Self {
        let timestamp = OffsetDateTime::now_utc();
        let formatted_timestamp = format_timestamp(timestamp);

        Self {
            sequence: 0,
            formatted_timestamp,
            level,
            target: target.to_string(),
            message,
            fields,
        }
    }

    fn assign_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LogField {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Default)]
struct LogVisitor {
    message: Option<String>,
    fields: Vec<LogField>,
}

impl Visit for LogVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.push(LogField {
                name: field.name().to_string(),
                value: value.to_string(),
            });
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push(LogField {
                name: field.name().to_string(),
                value,
            });
        }
    }
}

struct LogStoreLayer {
    store: LogStore,
}

impl<S> Layer<S> for LogStoreLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);

        let message = visitor
            .message
            .unwrap_or_else(|| "(no message)".to_string());
        let entry = LogEntry::new(
            *metadata.level(),
            metadata.target(),
            message,
            visitor.fields,
        );

        self.store.push(entry);
    }
}

const DISPLAY_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");

fn format_timestamp(timestamp: OffsetDateTime) -> String {
    timestamp
        .format(DISPLAY_FORMAT)
        .unwrap_or_else(|_| timestamp.to_string())
}
