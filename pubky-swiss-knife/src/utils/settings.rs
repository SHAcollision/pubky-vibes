use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use dioxus::prelude::{ReadableExt, Signal, WritableExt};
use serde::{Deserialize, Serialize};

use crate::NetworkMode;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub network_mode: NetworkMode,
    #[serde(default)]
    pub homeserver: String,
    #[serde(default)]
    pub signup_code: String,
    #[serde(default = "default_capabilities")]
    pub token_capabilities: String,
    #[serde(default = "default_capabilities")]
    pub auth_capabilities: String,
    #[serde(default)]
    pub auth_relay: String,
    #[serde(default)]
    pub auth_request: String,
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
    #[serde(default)]
    pub storage_body: String,
    #[serde(default)]
    pub public_resource: String,
    #[serde(default = "default_http_method")]
    pub http_method: String,
    #[serde(default = "default_http_url")]
    pub http_url: String,
    #[serde(default)]
    pub http_headers: String,
    #[serde(default)]
    pub http_body: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            network_mode: NetworkMode::Mainnet,
            homeserver: String::new(),
            signup_code: String::new(),
            token_capabilities: default_capabilities(),
            auth_capabilities: default_capabilities(),
            auth_relay: String::new(),
            auth_request: String::new(),
            storage_path: default_storage_path(),
            storage_body: String::new(),
            public_resource: String::new(),
            http_method: default_http_method(),
            http_url: default_http_url(),
            http_headers: String::new(),
            http_body: String::new(),
        }
    }
}

impl AppSettings {
    pub fn load_or_default() -> Self {
        match Self::try_load() {
            Ok(settings) => settings,
            Err(err) => {
                eprintln!("Failed to load settings: {err}");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path =
            config_file_path().ok_or_else(|| anyhow!("Unable to determine config directory"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
        let payload = serde_json::to_string_pretty(self)?;
        fs::write(&path, payload)
            .with_context(|| format!("failed to write settings file {}", path.display()))?;
        Ok(())
    }

    fn try_load() -> Result<Self> {
        let Some(path) = config_file_path() else {
            return Ok(Self::default());
        };
        let contents = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => return Ok(Self::default()),
                _ => {
                    return Err(err).with_context(|| {
                        format!("failed to read settings file {}", path.display())
                    });
                }
            },
        };
        let mut settings: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse settings file {}", path.display()))?;
        settings.normalize();
        Ok(settings)
    }

    fn normalize(&mut self) {
        if self.token_capabilities.trim().is_empty() {
            self.token_capabilities = default_capabilities();
        }
        if self.auth_capabilities.trim().is_empty() {
            self.auth_capabilities = default_capabilities();
        }
        if self.storage_path.trim().is_empty() {
            self.storage_path = default_storage_path();
        }
        if self.http_method.trim().is_empty() {
            self.http_method = default_http_method();
        }
        if self.http_url.trim().is_empty() {
            self.http_url = default_http_url();
        }
    }
}

#[derive(Clone)]
pub struct SettingsWriter {
    sender: mpsc::Sender<AppSettings>,
}

impl SettingsWriter {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<AppSettings>();
        thread::spawn(move || run_writer(receiver));
        Self { sender }
    }

    pub fn schedule_save(&self, settings: AppSettings) {
        let _ = self.sender.send(settings);
    }
}

pub fn persist_update(
    mut settings: Signal<AppSettings>,
    writer: Signal<SettingsWriter>,
    apply: impl FnOnce(&mut AppSettings),
) {
    let snapshot = {
        let mut guard = settings.write();
        apply(&mut guard);
        guard.clone()
    };
    let handle = writer.read().clone();
    handle.schedule_save(snapshot);
}

fn run_writer(receiver: mpsc::Receiver<AppSettings>) {
    use std::sync::mpsc::RecvTimeoutError;

    loop {
        let Ok(mut latest) = receiver.recv() else {
            break;
        };

        loop {
            match receiver.recv_timeout(DEBOUNCE_DURATION) {
                Ok(next) => latest = next,
                Err(RecvTimeoutError::Timeout) => {
                    if let Err(err) = latest.save() {
                        eprintln!("Failed to persist settings: {err}");
                    }
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if let Err(err) = latest.save() {
                        eprintln!("Failed to persist settings: {err}");
                    }
                    return;
                }
            }
        }
    }
}

fn config_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("pubky-swiss-knife").join("settings.json"))
}

fn default_capabilities() -> String {
    String::from("/:rw")
}

fn default_storage_path() -> String {
    String::from("/pub/")
}

fn default_http_method() -> String {
    String::from("GET")
}

fn default_http_url() -> String {
    String::from("https://")
}
