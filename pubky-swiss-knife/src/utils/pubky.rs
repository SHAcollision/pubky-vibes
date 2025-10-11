use std::sync::Arc;

use anyhow::{Result, anyhow};
use pubky::Pubky;

use crate::app::NetworkMode;

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

    pub fn facade(&self) -> Option<Arc<Pubky>> {
        match &self.status {
            PubkyFacadeStatus::Ready(facade) => Some(facade.clone()),
            _ => None,
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

pub async fn build_pubky_facade(mode: NetworkMode) -> Result<Arc<Pubky>> {
    let facade = tokio::task::spawn_blocking(move || match mode {
        NetworkMode::Mainnet => Pubky::new(),
        NetworkMode::Testnet => Pubky::testnet(),
    })
    .await
    .map_err(|err| anyhow!("Failed to join Pubky build task: {err}"))??;

    Ok(Arc::new(facade))
}
