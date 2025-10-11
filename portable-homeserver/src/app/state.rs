use std::{fmt, path::PathBuf, sync::Arc};

use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ServerStatus {
    Idle,
    Starting,
    Running(ServerInfo),
    Stopping,
    Error(String),
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ServerInfo {
    pub(crate) public_key: String,
    pub(crate) admin_url: String,
    pub(crate) icann_http_url: String,
    pub(crate) pubky_url: String,
    pub(crate) network: NetworkProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NetworkProfile {
    Mainnet,
    Testnet,
}

impl NetworkProfile {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Mainnet => "Mainnet",
            Self::Testnet => "Static Testnet",
        }
    }
}

impl fmt::Display for NetworkProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum RunningServer {
    Mainnet(Arc<HomeserverSuite>),
    Testnet(Arc<StaticTestnet>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StartSpec {
    Mainnet { data_dir: PathBuf },
    Testnet,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartValidationError {
    MissingDataDir,
}

impl fmt::Display for StartValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartValidationError::MissingDataDir => f.write_str(
                "Please provide a directory where the homeserver can persist its state.",
            ),
        }
    }
}

pub(crate) fn resolve_start_spec(
    network: NetworkProfile,
    data_dir: &str,
) -> Result<StartSpec, StartValidationError> {
    match network {
        NetworkProfile::Mainnet => {
            let trimmed = data_dir.trim();
            if trimmed.is_empty() {
                return Err(StartValidationError::MissingDataDir);
            }

            Ok(StartSpec::Mainnet {
                data_dir: PathBuf::from(trimmed),
            })
        }
        NetworkProfile::Testnet => Ok(StartSpec::Testnet),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_validation_error_formats_helpfully() {
        assert_eq!(
            StartValidationError::MissingDataDir.to_string(),
            "Please provide a directory where the homeserver can persist its state."
        );
    }
}
