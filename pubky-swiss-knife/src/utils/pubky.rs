use std::sync::Arc;

use anyhow::{Result, anyhow};
use dioxus::prelude::{ReadableExt, Signal, WritableExt};
use pubky::Pubky;

use crate::app::NetworkMode;
use crate::utils::logging::ActivityLog;

#[derive(Clone)]
pub struct PubkyFacadeState {
    pub network: NetworkMode,
    pub status: PubkyFacadeStatus,
}

#[derive(Clone)]
pub enum PubkyFacadeStatus {
    Loading,
    Ready(Arc<Pubky>),
    Error(String),
}

impl PubkyFacadeState {
    pub fn loading(network: NetworkMode) -> Self {
        Self {
            network,
            status: PubkyFacadeStatus::Loading,
        }
    }

    pub fn ready(network: NetworkMode, facade: Arc<Pubky>) -> Self {
        Self {
            network,
            status: PubkyFacadeStatus::Ready(facade),
        }
    }

    pub fn error(network: NetworkMode, message: impl Into<String>) -> Self {
        Self {
            network,
            status: PubkyFacadeStatus::Error(message.into()),
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.status, PubkyFacadeStatus::Loading)
    }

    pub fn error_message(&self) -> Option<&str> {
        match &self.status {
            PubkyFacadeStatus::Error(message) => Some(message.as_str()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct PubkyFacadeHandle {
    state: Signal<PubkyFacadeState>,
}

impl PubkyFacadeHandle {
    pub fn new(state: Signal<PubkyFacadeState>) -> Self {
        Self { state }
    }

    pub fn snapshot(&self) -> PubkyFacadeState {
        self.state.read().clone()
    }

    pub fn set(&self, next: PubkyFacadeState) {
        let mut setter = self.state.clone();
        setter.set(next);
    }

    pub fn ensure_ready(&self) -> Result<Arc<Pubky>, PubkyFacadeReadiness> {
        let snapshot = self.state.read().clone();
        match snapshot.status {
            PubkyFacadeStatus::Ready(facade) => Ok(facade),
            PubkyFacadeStatus::Loading => Err(PubkyFacadeReadiness::Loading(snapshot.network)),
            PubkyFacadeStatus::Error(message) => {
                Err(PubkyFacadeReadiness::Failed(snapshot.network, message))
            }
        }
    }

    pub fn ready_or_log(&self, logs: &ActivityLog) -> Option<Arc<Pubky>> {
        match self.ensure_ready() {
            Ok(facade) => Some(facade),
            Err(PubkyFacadeReadiness::Loading(_)) => {
                logs.info("Pubky facade is still starting up. Try again shortly.");
                None
            }
            Err(PubkyFacadeReadiness::Failed(_, message)) => {
                logs.error(format!("Pubky facade unavailable: {message}"));
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum PubkyFacadeReadiness {
    Loading(NetworkMode),
    Failed(NetworkMode, String),
}

impl std::fmt::Display for PubkyFacadeReadiness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PubkyFacadeReadiness::Loading(network) => {
                write!(f, "Pubky facade for {} is still starting", network.label())
            }
            PubkyFacadeReadiness::Failed(network, message) => {
                write!(f, "Pubky facade for {} failed: {message}", network.label())
            }
        }
    }
}

impl std::error::Error for PubkyFacadeReadiness {}

pub async fn build_pubky_facade(mode: NetworkMode) -> Result<Arc<Pubky>> {
    let facade = tokio::task::spawn_blocking(move || match mode {
        NetworkMode::Mainnet => Pubky::new(),
        NetworkMode::Testnet => Pubky::testnet(),
    })
    .await
    .map_err(|err| anyhow!("Failed to join Pubky build task: {err}"))??;

    Ok(Arc::new(facade))
}
