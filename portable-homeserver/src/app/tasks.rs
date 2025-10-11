use std::sync::Arc;

use anyhow::Context;
use dioxus::prelude::{ReadableExt, WritableExt, spawn};
use dioxus::signals::{Signal, SignalData, Storage};
use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;

use super::state::{NetworkProfile, RunningServer, ServerInfo, ServerStatus, StartSpec};

pub(crate) fn stop_current_server<S1, S2>(
    mut status_signal: Signal<ServerStatus, S1>,
    mut suite_signal: Signal<Option<RunningServer>, S2>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    let was_active = matches!(
        *status_signal.peek(),
        ServerStatus::Running(_) | ServerStatus::Starting
    );

    if was_active {
        *status_signal.write() = ServerStatus::Stopping;
    }

    suite_signal.write().take();
    *status_signal.write() = ServerStatus::Idle;
}

pub(crate) fn spawn_start_task<S1, S2>(
    start_spec: StartSpec,
    mut status_signal: Signal<ServerStatus, S1>,
    suite_signal: Signal<Option<RunningServer>, S2>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    *status_signal.write() = ServerStatus::Starting;

    let mut status_for_task = status_signal.clone();
    let mut suite_for_task = suite_signal.clone();

    spawn(async move {
        let result: anyhow::Result<(RunningServer, ServerInfo)> = async {
            match start_spec {
                StartSpec::Mainnet { data_dir } => {
                    tokio::fs::create_dir_all(&data_dir)
                        .await
                        .with_context(|| {
                            format!("Failed to create data directory at {}", data_dir.display())
                        })?;

                    let server =
                        HomeserverSuite::start_with_persistent_data_dir_path(data_dir.clone())
                            .await
                            .with_context(|| {
                                format!(
                                    "HomeserverSuite::start_with_persistent_data_dir_path({})",
                                    data_dir.display()
                                )
                            })?;

                    let info = ServerInfo {
                        public_key: server.public_key().to_string(),
                        admin_url: format!("http://{}", server.admin().listen_socket()),
                        icann_http_url: server.icann_http_url().to_string(),
                        pubky_url: server.pubky_url().to_string(),
                        network: NetworkProfile::Mainnet,
                    };

                    Ok((RunningServer::Mainnet(Arc::new(server)), info))
                }
                StartSpec::Testnet => {
                    let static_net = StaticTestnet::start()
                        .await
                        .context("StaticTestnet::start()")?;
                    let static_net = Arc::new(static_net);
                    let homeserver = static_net.homeserver();
                    let info = ServerInfo {
                        public_key: homeserver.public_key().to_string(),
                        admin_url: format!("http://{}", homeserver.admin().listen_socket()),
                        icann_http_url: homeserver.icann_http_url().to_string(),
                        pubky_url: homeserver.pubky_url().to_string(),
                        network: NetworkProfile::Testnet,
                    };

                    Ok((RunningServer::Testnet(static_net), info))
                }
            }
        }
        .await;

        match result {
            Ok((suite, info)) => {
                *suite_for_task.write() = Some(suite);
                *status_for_task.write() = ServerStatus::Running(info);
            }
            Err(err) => {
                *status_for_task.write() =
                    ServerStatus::Error(format!("Failed to start the homeserver: {err:?}"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use super::super::state::{NetworkProfile, StartValidationError, resolve_start_spec};

    #[test]
    fn resolves_mainnet_start_spec_with_trimmed_path() {
        let spec = resolve_start_spec(NetworkProfile::Mainnet, "  /tmp/pubky  ");

        assert_eq!(
            spec,
            Ok(StartSpec::Mainnet {
                data_dir: PathBuf::from("/tmp/pubky"),
            })
        );
    }

    #[test]
    fn rejects_mainnet_start_without_path() {
        let err = resolve_start_spec(NetworkProfile::Mainnet, "   ")
            .expect_err("missing directories must error");

        assert_eq!(err, StartValidationError::MissingDataDir);
        assert_eq!(
            err.to_string(),
            "Please provide a directory where the homeserver can persist its state."
        );
    }

    #[test]
    fn resolves_testnet_start_spec() {
        let spec = resolve_start_spec(NetworkProfile::Testnet, "ignored");
        assert_eq!(spec, Ok(StartSpec::Testnet));
    }
}
