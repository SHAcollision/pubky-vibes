use std::sync::Arc;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[cfg(target_os = "android")]
use directories::ProjectDirs;

use anyhow::{Context, Result};
use dioxus::prelude::{ReadableExt, WritableExt, spawn};
use dioxus::signals::{Signal, SignalData, Storage};
use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;
#[cfg(target_os = "android")]
use tempfile::TempDir;
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
        if let Some(server) = maybe_server
            && let Err(err) = shutdown_running_server(server).await
        {
            error!(?err, "failed to stop homeserver");
            *status_for_task.write() =
                ServerStatus::Error(format!("Failed to stop the homeserver cleanly: {err}"));
            return;
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
            #[cfg(target_os = "android")]
            ensure_android_temp_dir()
                .context("Failed to prepare temporary storage for Android testnet")?;

            let static_net = StaticTestnet::start()
                .await
                .context("StaticTestnet::start()")?;
            let homeserver = static_net.homeserver();
            let info = server_info_from_suite(homeserver, NetworkProfile::Testnet);

            Ok((RunningServer::Testnet(Arc::new(static_net)), info))
        }
    }
}

#[cfg(target_os = "android")]
fn ensure_android_temp_dir() -> Result<()> {
    if let Some(current) = env::var_os("TMPDIR").filter(|value| !value.is_empty()) {
        ensure_dir_exists(Path::new(&current))?;
        return Ok(());
    }

    static ANDROID_TESTNET_TMP: OnceLock<TempDir> = OnceLock::new();

    let dir_path = if let Some(existing) = ANDROID_TESTNET_TMP.get() {
        existing.path().to_path_buf()
    } else {
        let root = android_cache_root().context("Failed to locate Android cache directory")?;
        let created = tempfile::Builder::new()
            .prefix("pubky-testnet-")
            .tempdir_in(root)
            .context("Failed to allocate Android cache-backed tempdir for testnet")?;
        let path = created.path().to_path_buf();
        let _ = ANDROID_TESTNET_TMP.set(created);
        path
    };

    set_android_tmp_vars(&dir_path);

    Ok(())
}

#[cfg(target_os = "android")]
fn android_cache_root() -> Result<PathBuf> {
    if let Some(project_dirs) = ProjectDirs::from("io", "Pubky", "PortableHomeserver") {
        let cache_dir = project_dirs.cache_dir();
        fs::create_dir_all(cache_dir).with_context(|| {
            format!(
                "Failed to create Android cache directory at {}",
                cache_dir.display()
            )
        })?;
        return Ok(cache_dir.to_path_buf());
    }

    let fallback = env::temp_dir();
    ensure_dir_exists(&fallback)?;
    Ok(fallback)
}

#[cfg(target_os = "android")]
fn ensure_dir_exists(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| {
        format!(
            "Failed to ensure Android directory {} exists",
            path.display()
        )
    })
}

#[cfg(target_os = "android")]
fn set_android_tmp_vars(path: &Path) {
    // SAFETY: Setting process environment variables is inherently unsafe on Android because
    // it crosses the FFI boundary into libc. We only write ASCII variable names with a
    // platform-provided cache directory that we just created successfully, which satisfies
    // the API's contract.
    unsafe {
        for var in ["TMPDIR", "TMP", "TEMP"] {
            env::set_var(var, path);
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
