use std::sync::Arc;

use anyhow::{Context, Result};
use dioxus::prelude::{ReadableExt, WritableExt, spawn};
use dioxus::signals::{Signal, SignalData, Storage};
use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;
use tokio::time::{Duration, sleep};
use tracing::error;

use super::state::{NetworkProfile, RunningServer, ServerInfo, ServerStatus, StartSpec};

/// Stop the currently running homeserver (if any) and transition the UI once the
/// shutdown completes. Optionally runs a callback after the shutdown finishes or
/// immediately if there was nothing to stop.
pub(crate) fn stop_current_server<S1, S2, F>(
    mut status_signal: Signal<ServerStatus, S1>,
    mut suite_signal: Signal<Option<RunningServer>, S2>,
    on_stopped: Option<F>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
    F: FnOnce() + 'static,
{
    let should_stop = matches!(
        *status_signal.peek(),
        ServerStatus::Running(_) | ServerStatus::Starting | ServerStatus::Stopping
    );

    if !should_stop {
        suite_signal.write().take();
        *status_signal.write() = ServerStatus::Idle;

        if let Some(on_stopped) = on_stopped {
            on_stopped();
        }

        return;
    }

    *status_signal.write() = ServerStatus::Stopping;

    let maybe_server = suite_signal.write().take();
    let mut status_for_task = status_signal;
    let mut on_stopped = on_stopped;

    spawn(async move {
        if let Some(server) = maybe_server {
            if let Err(err) = shutdown_running_server(server).await {
                error!(?err, "failed to stop homeserver");
                *status_for_task.write() =
                    ServerStatus::Error(format!("Failed to stop the homeserver cleanly: {err}"));
                return;
            }
        }

        *status_for_task.write() = ServerStatus::Idle;

        if let Some(on_stopped) = on_stopped.take() {
            on_stopped();
        }
    });
}

/// Spawn the async task that launches a homeserver and keeps the UI updated with
/// progress and errors.
pub(crate) fn spawn_start_task<S1, S2>(
    start_spec: StartSpec,
    mut status_signal: Signal<ServerStatus, S1>,
    suite_signal: Signal<Option<RunningServer>, S2>,
) where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    *status_signal.write() = ServerStatus::Starting;

    let mut status_for_task = status_signal;
    let mut suite_for_task = suite_signal;

    spawn(async move {
        let result = start_server(start_spec).await;
        match result {
            Ok((suite, info)) => {
                *suite_for_task.write() = Some(suite);
                *status_for_task.write() = ServerStatus::Running(info);
            }
            Err(err) => {
                error!(?err, "failed to start homeserver");
                *status_for_task.write() = ServerStatus::Error(err.to_string());
            }
        }
    });
}

async fn shutdown_running_server(server: RunningServer) -> Result<()> {
    match server {
        RunningServer::Mainnet(handle) => {
            handle.core().shutdown();
            // Give the runtime a moment to flush sockets.
            sleep(Duration::from_millis(100)).await;
            drop(handle);
        }
        RunningServer::Testnet(testnet) => {
            testnet.homeserver().core().shutdown();
            sleep(Duration::from_millis(100)).await;
            drop(testnet);
        }
    }

    Ok(())
}

async fn start_server(start_spec: StartSpec) -> Result<(RunningServer, ServerInfo)> {
    match start_spec {
        StartSpec::Mainnet { data_dir } => {
            tokio::fs::create_dir_all(&data_dir)
                .await
                .with_context(|| {
                    format!("Failed to create data directory at {}", data_dir.display())
                })?;

            let server = HomeserverSuite::start_with_persistent_data_dir_path(data_dir.clone())
                .await
                .with_context(|| {
                    format!(
                        "HomeserverSuite::start_with_persistent_data_dir_path({})",
                        data_dir.display()
                    )
                })?;

            let info = server_info_from_suite(&server, NetworkProfile::Mainnet);

            Ok((RunningServer::Mainnet(Arc::new(server)), info))
        }
        StartSpec::Testnet => {
            let static_net = StaticTestnet::start()
                .await
                .context("StaticTestnet::start()")?;
            let homeserver = static_net.homeserver();
            let info = server_info_from_suite(homeserver, NetworkProfile::Testnet);

            Ok((RunningServer::Testnet(Arc::new(static_net)), info))
        }
    }
}

fn server_info_from_suite(suite: &HomeserverSuite, network: NetworkProfile) -> ServerInfo {
    ServerInfo {
        public_key: suite.public_key().to_string(),
        admin_url: format!("http://{}", suite.admin().listen_socket()),
        icann_http_url: suite.icann_http_url().to_string(),
        pubky_url: suite.pubky_url().to_string(),
        network,
    }
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
