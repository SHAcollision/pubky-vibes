use std::{
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    sync::Arc,
    time::Instant,
};

use anyhow::{Context, Result, anyhow};
use dioxus::prelude::{ReadableExt, WritableExt, spawn};
use dioxus::signals::{Signal, SignalData, Storage};
use pubky_homeserver::HomeserverSuite;
use pubky_testnet::StaticTestnet;
use tokio::time::{Duration, sleep};
use tracing::{error, warn};

use super::state::{NetworkProfile, RunningServer, ServerInfo, ServerStatus, StartSpec};

const STATIC_TESTNET_MAX_ADDR_IN_USE_RETRIES: usize = 5;

const STATIC_TESTNET_PORTS: [u16; 6] = [15411, 15412, 6286, 6287, 6288, 6881];

#[cfg(test)]
const STATIC_TESTNET_PORT_RELEASE_TIMEOUT_MS: u64 = 1_000;

#[cfg(not(test))]
const STATIC_TESTNET_PORT_RELEASE_TIMEOUT_MS: u64 = 5_000;

#[cfg(test)]
const STATIC_TESTNET_PORT_POLL_INTERVAL_MS: u64 = 10;

#[cfg(not(test))]
const STATIC_TESTNET_PORT_POLL_INTERVAL_MS: u64 = 100;

#[cfg(test)]
const STATIC_TESTNET_RETRY_DELAY_MS: u64 = 0;

#[cfg(not(test))]
const STATIC_TESTNET_RETRY_DELAY_MS: u64 = 200;

async fn retry_addr_in_use<F, Fut, T>(mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_addr_in_use_error: Option<anyhow::Error> = None;

    for attempt in 0..=STATIC_TESTNET_MAX_ADDR_IN_USE_RETRIES {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt < STATIC_TESTNET_MAX_ADDR_IN_USE_RETRIES && is_addr_in_use_error(&err) {
                    warn!(
                        attempt = attempt + 1,
                        delay_ms = STATIC_TESTNET_RETRY_DELAY_MS,
                        "Static testnet start failed because a port is still in use; retrying",
                    );
                    sleep(Duration::from_millis(STATIC_TESTNET_RETRY_DELAY_MS)).await;
                    last_addr_in_use_error = Some(err);
                    continue;
                }

                return Err(err);
            }
        }
    }

    Err(last_addr_in_use_error.unwrap_or_else(|| {
        anyhow!(
            "Port remained in use after {} attempts",
            STATIC_TESTNET_MAX_ADDR_IN_USE_RETRIES + 1
        )
    }))
}

fn is_addr_in_use_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        if let Some(io_err) = cause.downcast_ref::<io::Error>() {
            io_err.kind() == io::ErrorKind::AddrInUse
        } else {
            cause.to_string().contains("Address already in use")
        }
    })
}

async fn wait_for_static_testnet_ports_to_release() -> Result<()> {
    wait_for_ports_to_release(
        &STATIC_TESTNET_PORTS,
        Duration::from_millis(STATIC_TESTNET_PORT_RELEASE_TIMEOUT_MS),
        Duration::from_millis(STATIC_TESTNET_PORT_POLL_INTERVAL_MS),
    )
    .await
}

async fn wait_for_ports_to_release(
    ports: &[u16],
    timeout: Duration,
    poll_interval: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_blocked_port = None;

    loop {
        let mut all_ports_free = true;

        for &port in ports {
            match TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))) {
                Ok(listener) => drop(listener),
                Err(err) => {
                    if err.kind() == io::ErrorKind::AddrInUse {
                        last_blocked_port = Some(port);
                        all_ports_free = false;
                        break;
                    }

                    return Err(err)
                        .with_context(|| format!("Failed to probe port {port} availability"));
                }
            }
        }

        if all_ports_free {
            return Ok(());
        }

        if Instant::now() >= deadline {
            let port = last_blocked_port
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(anyhow!(
                "Ports {:?} remained bound after shutdown; last blocked port: {}",
                ports,
                port
            ));
        }

        sleep(poll_interval).await;
    }
}

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
/// progress and errors. Returns `true` when a new start task was enqueued.
pub(crate) fn spawn_start_task<S1, S2>(
    start_spec: StartSpec,
    status_signal: Signal<ServerStatus, S1>,
    suite_signal: Signal<Option<RunningServer>, S2>,
) -> bool
where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
{
    spawn_start_task_with(start_spec, status_signal, suite_signal, start_server)
}

fn spawn_start_task_with<S1, S2, F, Fut>(
    start_spec: StartSpec,
    mut status_signal: Signal<ServerStatus, S1>,
    suite_signal: Signal<Option<RunningServer>, S2>,
    start_fn: F,
) -> bool
where
    S1: Storage<SignalData<ServerStatus>> + 'static,
    S2: Storage<SignalData<Option<RunningServer>>> + 'static,
    F: FnOnce(StartSpec) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(RunningServer, ServerInfo)>> + Send + 'static,
{
    if matches!(
        *status_signal.peek(),
        ServerStatus::Starting | ServerStatus::Running(_) | ServerStatus::Stopping
    ) {
        return false;
    }

    *status_signal.write() = ServerStatus::Starting;

    let mut status_for_task = status_signal;
    let mut suite_for_task = suite_signal;
    let start_future = start_fn(start_spec);

    spawn(async move {
        let result = start_future.await;
        match result {
            Ok((suite, info)) => {
                *suite_for_task.write() = Some(suite);
                *status_for_task.write() = ServerStatus::Running(info);
            }
            Err(err) => {
                error!(?err, "failed to start homeserver");
                *status_for_task.write() = ServerStatus::Error(format!("{err:#}"));
            }
        }
    });

    true
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
            testnet.pkarr_relay().shutdown();

            sleep(Duration::from_millis(100)).await;

            let strong_references = Arc::strong_count(&testnet);
            drop(testnet);

            if strong_references > 1 {
                warn!(
                    strong_references,
                    "Static testnet shutdown still has outstanding references"
                );
            }

            wait_for_static_testnet_ports_to_release().await?;
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
            let static_net = retry_addr_in_use(StaticTestnet::start)
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
    use std::{
        io,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::super::state::{NetworkProfile, StartValidationError, resolve_start_spec};
    use anyhow::{Result, anyhow};
    use dioxus::core::{RuntimeGuard, ScopeId, VNode, VirtualDom};
    use dioxus::signals::Signal;

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

    fn empty_app() -> dioxus::core::Element {
        VNode::empty()
    }

    #[test]
    fn ignores_additional_start_requests_while_starting() {
        let dom = VirtualDom::new(empty_app);
        let runtime = dom.runtime();
        let _guard = RuntimeGuard::new(runtime);

        let status = Signal::new_in_scope(ServerStatus::Starting, ScopeId::ROOT);
        let running = Signal::new_in_scope(None::<RunningServer>, ScopeId::ROOT);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_fn = attempts.clone();

        let launched = spawn_start_task_with(StartSpec::Testnet, status, running, move |_spec| {
            attempts_for_fn.fetch_add(1, Ordering::SeqCst);
            async move { Err(anyhow!("start task should not be invoked")) }
        });

        assert!(!launched, "second launch attempt must be ignored");
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            0,
            "start task must not run"
        );
    }

    #[tokio::test]
    async fn retries_addr_in_use_errors_until_success() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_closure = attempts.clone();

        let result: Result<i32> = retry_addr_in_use(move || {
            let attempts_for_iteration = attempts_for_closure.clone();

            async move {
                let attempt = attempts_for_iteration.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(anyhow::Error::from(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "still busy",
                    )))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(result.expect("should succeed after retries"), 42);
    }

    #[tokio::test]
    async fn does_not_retry_for_non_addr_in_use_errors() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_closure = attempts.clone();

        let result: Result<()> = retry_addr_in_use(move || {
            let attempts_for_iteration = attempts_for_closure.clone();

            async move {
                attempts_for_iteration.fetch_add(1, Ordering::SeqCst);
                Err(anyhow!("boom"))
            }
        })
        .await;

        assert!(result.is_err(), "non addr-in-use errors should bubble");
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gives_up_after_configured_addr_in_use_retries() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_closure = attempts.clone();

        let result: Result<()> = retry_addr_in_use(move || {
            let attempts_for_iteration = attempts_for_closure.clone();

            async move {
                attempts_for_iteration.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::Error::from(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "still busy",
                )))
            }
        })
        .await;

        assert!(result.is_err(), "exhausted retries should return an error");
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            STATIC_TESTNET_MAX_ADDR_IN_USE_RETRIES + 1
        );
    }

    #[tokio::test]
    async fn static_testnet_can_restart_after_shutdown() {
        let initial = StaticTestnet::start()
            .await
            .expect("initial static testnet should start");

        shutdown_running_server(RunningServer::Testnet(Arc::new(initial)))
            .await
            .expect("static testnet shutdown should succeed");

        let restarted = StaticTestnet::start()
            .await
            .expect("static testnet should restart cleanly");

        shutdown_running_server(RunningServer::Testnet(Arc::new(restarted)))
            .await
            .expect("shutdown after restart should succeed");
    }
}
