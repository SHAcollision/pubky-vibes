use std::{fmt, path::PathBuf, sync::Arc};

use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;

/// High level lifecycle representation for the homeserver UI.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ServerStatus {
    /// No background process is currently running.
    Idle,
    /// A start request is in-flight.
    Starting,
    /// A homeserver (or bundled testnet) is running and ready for interaction.
    Running(ServerInfo),
    /// A stop request is in-flight.
    Stopping,
    /// Something failed; the string is a user-facing explanation rendered in the UI.
    Error(String),
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Snapshot of the information we display once a server is online.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ServerInfo {
    pub(crate) public_key: String,
    pub(crate) admin_url: String,
    pub(crate) icann_http_url: String,
    pub(crate) pubky_url: String,
    pub(crate) network: NetworkProfile,
}

/// Supported network modes for the UI toggle.
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

/// Handle to the background process that keeps the homeserver alive.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum RunningServer {
    Mainnet(Arc<HomeserverSuite>),
    Testnet(Arc<StaticTestnet>),
}

/// Parameters required to start a network profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StartSpec {
    Mainnet { data_dir: PathBuf },
    Testnet,
}

/// Validation errors raised before we try to spawn any background process.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartValidationError {
    /// User did not supply a directory for persistent state.
    MissingDataDir,
    /// The supplied path exists but is not a directory.
    NotADirectory(PathBuf),
}

impl fmt::Display for StartValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartValidationError::MissingDataDir => f.write_str(
                "Please provide a directory where the homeserver can persist its state.",
            ),
            StartValidationError::NotADirectory(path) => write!(
                f,
                "{} points to a file. Please pick a directory so we can store the homeserver state.",
                path.display()
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

            let path = PathBuf::from(trimmed);
            if path.exists() && !path.is_dir() {
                return Err(StartValidationError::NotADirectory(path));
            }

            Ok(StartSpec::Mainnet { data_dir: path })
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

    #[test]
    fn start_validation_detects_non_directories() {
        let file = tempfile::NamedTempFile::new().expect("create temp file");
        let path_str = file.path().to_string_lossy().to_string();

        let err = resolve_start_spec(NetworkProfile::Mainnet, &path_str)
            .expect_err("files should be rejected");

        assert!(matches!(err, StartValidationError::NotADirectory(_)));
        assert!(
            err.to_string()
                .contains(file.path().to_string_lossy().as_ref())
        );
    }

    #[test]
    fn resolves_testnet_start_spec() {
        let spec = resolve_start_spec(NetworkProfile::Testnet, "ignored");
        assert_eq!(spec, Ok(StartSpec::Testnet));
    }

    #[test]
    fn resolves_mainnet_start_spec_with_trimmed_path() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path_str = format!("  {}  ", temp_dir.path().display());

        let spec = resolve_start_spec(NetworkProfile::Mainnet, &path_str)
            .expect("valid directory should resolve");

        assert_eq!(
            spec,
            StartSpec::Mainnet {
                data_dir: temp_dir.path().to_path_buf()
            }
        );
    }
}
