use std::{sync::LazyLock, time::Instant};

use dioxus::events::{FormEvent, MouseEvent};
use dioxus::prelude::{spawn, *};
use dioxus::signals::{Signal, SyncStorage};
use pubky_homeserver::SignupMode;
use tokio::time::{Duration, sleep};

use super::admin::{self, AdminInfo};
use super::config::{
    ConfigFeedback, ConfigForm, ConfigState, config_state_from_dir, default_data_dir,
    load_config_form_from_dir, modify_config_form, persist_config_form,
};
use super::state::{NetworkProfile, RunningServer, ServerStatus, resolve_start_spec};
use super::status::{StatusCopy, StatusDetails, status_copy, status_details};
use super::style::{LOGO_DATA_URI, STYLE};
use super::tasks::{spawn_start_task, stop_current_server};

#[derive(Clone, Debug)]
enum FetchState<T> {
    Idle,
    Loading,
    Loaded(T),
    Error(String),
}

impl<T> Default for FetchState<T> {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActionFeedback {
    Info(String),
    Success(String),
    Error(String),
}

impl ActionFeedback {
    fn class(&self) -> &'static str {
        match self {
            ActionFeedback::Info(_) => "info",
            ActionFeedback::Success(_) => "success",
            ActionFeedback::Error(_) => "error",
        }
    }

    fn message(&self) -> &str {
        match self {
            ActionFeedback::Info(message)
            | ActionFeedback::Success(message)
            | ActionFeedback::Error(message) => message.as_str(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DeleteEntryFormState {
    pubkey: String,
    entry_path: String,
    feedback: Option<ActionFeedback>,
    in_flight: bool,
}

#[derive(Clone, Debug, Default)]
struct DisableUserFormState {
    pubkey: String,
    feedback: Option<ActionFeedback>,
    in_flight: bool,
}

#[derive(Clone, Debug)]
struct AdminPanelState {
    password: String,
    password_initialized: bool,
    info: FetchState<AdminInfo>,
    info_refresh_nonce: u64,
    signup_token: Option<String>,
    signup_feedback: Option<ActionFeedback>,
    signup_in_flight: bool,
    delete_form: DeleteEntryFormState,
    disable_form: DisableUserFormState,
}

impl Default for AdminPanelState {
    fn default() -> Self {
        Self {
            password: String::new(),
            password_initialized: false,
            info: FetchState::Idle,
            info_refresh_nonce: 1,
            signup_token: None,
            signup_feedback: None,
            signup_in_flight: false,
            delete_form: DeleteEntryFormState::default(),
            disable_form: DisableUserFormState::default(),
        }
    }
}

impl AdminPanelState {
    fn ensure_password(&mut self, fallback: String) {
        if !self.password_initialized {
            self.password = fallback;
            self.password_initialized = true;
        }
    }

    fn bump_info_refresh(&mut self) {
        self.info_refresh_nonce = self.info_refresh_nonce.wrapping_add(1);
    }
}

async fn poll_admin_info(
    status: Signal<ServerStatus, SyncStorage>,
    mut admin_state: Signal<AdminPanelState, SyncStorage>,
) {
    let mut last_nonce = 0;
    let mut last_admin_url: Option<String> = None;
    let mut last_fetch = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);

    loop {
        let status_snapshot = status.read().clone();
        let (password, nonce) = {
            let state = admin_state.read();
            (state.password.clone(), state.info_refresh_nonce)
        };

        match status_snapshot {
            ServerStatus::Running(info) => {
                let admin_url = info.admin_url.clone();
                let mut should_fetch = false;

                if last_admin_url.as_deref() != Some(admin_url.as_str()) {
                    should_fetch = true;
                    last_admin_url = Some(admin_url.clone());
                }

                if nonce != last_nonce {
                    should_fetch = true;
                    last_nonce = nonce;
                }

                if last_fetch.elapsed() >= Duration::from_secs(30) {
                    should_fetch = true;
                }

                if should_fetch {
                    if password.trim().is_empty() {
                        {
                            let mut state = admin_state.write();
                            state.info = FetchState::Error(
                                "Provide the admin password to load server stats.".into(),
                            );
                        }
                        last_fetch = Instant::now();
                    } else {
                        {
                            let mut state = admin_state.write();
                            state.info = FetchState::Loading;
                        }

                        let result = admin::fetch_info(&admin_url, &password).await;
                        match result {
                            Ok(info) => {
                                let mut state = admin_state.write();
                                state.info = FetchState::Loaded(info);
                            }
                            Err(err) => {
                                let mut state = admin_state.write();
                                state.info = FetchState::Error(format!(
                                    "Failed to load server stats: {}",
                                    err
                                ));
                            }
                        }

                        last_fetch = Instant::now();
                    }
                }
            }
            _ => {
                if last_admin_url.take().is_some() {
                    let mut state = admin_state.write();
                    state.info = FetchState::Idle;
                }
                last_fetch = Instant::now()
                    .checked_sub(Duration::from_secs(60))
                    .unwrap_or_else(Instant::now);
                last_nonce = 0;
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

fn sanitize_entry_target(pubkey: &str, entry_path: &str) -> Result<String, String> {
    let trimmed_pubkey = pubkey.trim();
    if trimmed_pubkey.is_empty() {
        return Err("Enter the tenant pubkey.".into());
    }

    let trimmed_path = entry_path.trim();
    if trimmed_path.is_empty() {
        return Err("Enter the entry path to delete.".into());
    }

    let sanitized = trimmed_path.trim_start_matches('/');
    if !sanitized.starts_with("pub/") {
        return Err("Entry path must begin with /pub/.".into());
    }

    Ok(format!("{}/{}", trimmed_pubkey, sanitized))
}

fn toggle_user_access(
    status: Signal<ServerStatus, SyncStorage>,
    mut admin_state: Signal<AdminPanelState, SyncStorage>,
    disable: bool,
) {
    let status_snapshot = status.read().clone();
    if let ServerStatus::Running(info) = status_snapshot {
        let admin_url = info.admin_url.clone();
        let (password, pubkey) = {
            let state = admin_state.read();
            (state.password.clone(), state.disable_form.pubkey.clone())
        };

        if password.trim().is_empty() {
            let mut state = admin_state.write();
            state.disable_form.feedback = Some(ActionFeedback::Error(
                "Provide the admin password to change user access.".into(),
            ));
            return;
        }

        if pubkey.trim().is_empty() {
            let mut state = admin_state.write();
            state.disable_form.feedback =
                Some(ActionFeedback::Error("Enter the user pubkey.".into()));
            return;
        }

        {
            let mut state = admin_state.write();
            state.disable_form.in_flight = true;
            let action_copy = if disable {
                "Disabling user…"
            } else {
                "Enabling user…"
            };
            state.disable_form.feedback = Some(ActionFeedback::Info(action_copy.into()));
        }

        let mut admin_state_task = admin_state.clone();
        spawn(async move {
            let result = admin::toggle_user_disabled(&admin_url, &password, &pubkey, disable).await;
            let mut state = admin_state_task.write();
            state.disable_form.in_flight = false;
            match result {
                Ok(()) => {
                    let success_copy = if disable {
                        "User disabled.".to_string()
                    } else {
                        "User enabled.".to_string()
                    };
                    state.disable_form.feedback = Some(ActionFeedback::Success(success_copy));
                    state.bump_info_refresh();
                }
                Err(err) => {
                    state.disable_form.feedback = Some(ActionFeedback::Error(format!(
                        "Failed to update user: {}",
                        err
                    )));
                }
            }
        });
    } else {
        let mut state = admin_state.write();
        state.disable_form.feedback = Some(ActionFeedback::Error(
            "Start the homeserver to change user access.".into(),
        ));
    }
}

#[component]
pub fn App() -> Element {
    let initial_data_dir = default_data_dir();
    let initial_config_state = config_state_from_dir(&initial_data_dir);

    let data_dir = use_signal_sync(|| initial_data_dir.clone());
    let status = use_signal_sync(ServerStatus::default);
    let running_server = use_signal_sync(|| Option::<RunningServer>::None);
    let network = use_signal_sync(|| NetworkProfile::Mainnet);
    let config_state = use_signal_sync(|| initial_config_state.clone());

    let status_for_admin = status.clone();
    let config_for_admin = config_state.clone();

    let status_snapshot = status.read().clone();
    let data_dir_snapshot = data_dir.read().clone();

    rsx! {
        style { "{STYLE}" }
        main { class: "app",
            AdminPanel { status: status_for_admin, config_state: config_for_admin }
            Hero {}
            ControlsPanel {
                data_dir: data_dir.clone(),
                network: network.clone(),
                config_state: config_state.clone(),
                status: status.clone(),
                running_server: running_server.clone()
            }
            StatusPanel { status: status_snapshot }
            FooterNotes { data_dir: data_dir_snapshot }
        }
    }
}

#[component]
fn AdminPanel(
    status: Signal<ServerStatus, SyncStorage>,
    config_state: Signal<ConfigState, SyncStorage>,
) -> Element {
    let mut admin_state = use_signal_sync(AdminPanelState::default);

    let config_password = {
        let guard = config_state.read();
        guard.form.admin_password.clone()
    };
    {
        let mut state = admin_state.write();
        state.ensure_password(config_password.clone());
    }

    let mut poller_started = use_signal_sync(|| false);
    if !*poller_started.read() {
        *poller_started.write() = true;
        let status_for_task = status.clone();
        let admin_state_for_task = admin_state.clone();
        spawn(async move {
            poll_admin_info(status_for_task, admin_state_for_task).await;
        });
    }

    let status_snapshot = status.read().clone();
    let admin_snapshot = admin_state.read().clone();

    let info_section = match &admin_snapshot.info {
        FetchState::Idle => match status_snapshot {
            ServerStatus::Running(_) => rsx! {
                div { class: "admin-info-message", "Waiting for the first stats update…" }
            },
            _ => {
                rsx! { div { class: "admin-info-message", "Start the homeserver to see live stats." } }
            }
        },
        FetchState::Loading => {
            rsx! { div { class: "admin-info-message", "Loading homeserver stats…" } }
        }
        FetchState::Loaded(info) => {
            let disabled_hint = if info.num_disabled_users > 0 {
                format!("{} disabled", info.num_disabled_users)
            } else {
                "All active".to_string()
            };
            let unused_hint = if info.num_unused_signup_codes > 0 {
                format!("{} unused", info.num_unused_signup_codes)
            } else {
                "None unused".to_string()
            };
            let disk_used = format!("{:.1} MB", info.total_disk_used_mb);

            rsx! {
                div { class: "admin-metrics-grid",
                    div { class: "admin-metric",
                        span { class: "metric-label", "Users" }
                        span { class: "metric-value", "{info.num_users}" }
                        span { class: "metric-hint", "{disabled_hint}" }
                    }
                    div { class: "admin-metric",
                        span { class: "metric-label", "Disk used" }
                        span { class: "metric-value", "{disk_used}" }
                        span { class: "metric-hint", "Includes all tenants" }
                    }
                    div { class: "admin-metric",
                        span { class: "metric-label", "Signup codes" }
                        span { class: "metric-value", "{info.num_signup_codes}" }
                        span { class: "metric-hint", "{unused_hint}" }
                    }
                }
            }
        }
        FetchState::Error(message) => {
            rsx! { div { class: "admin-feedback error", "{message}" } }
        }
    };

    let mut admin_state_for_password = admin_state.clone();
    let on_password_change = move |evt: FormEvent| {
        let mut state = admin_state_for_password.write();
        state.password = evt.value();
    };

    let mut admin_state_for_use_config = admin_state.clone();
    let config_state_for_use = config_state.clone();
    let on_use_config_password = move |_| {
        let fallback = {
            let guard = config_state_for_use.read();
            guard.form.admin_password.clone()
        };
        let mut state = admin_state_for_use_config.write();
        state.password = fallback;
        state.bump_info_refresh();
    };

    let mut admin_state_for_refresh = admin_state.clone();
    let on_refresh_info = move |_| {
        let mut state = admin_state_for_refresh.write();
        state.bump_info_refresh();
    };

    let status_for_token = status.clone();
    let mut admin_state_for_token = admin_state.clone();
    let on_generate_token = move |_| {
        let status_snapshot = status_for_token.read().clone();
        if let ServerStatus::Running(info) = status_snapshot {
            let admin_url = info.admin_url.clone();
            let password = {
                let state = admin_state_for_token.read();
                state.password.clone()
            };

            if password.trim().is_empty() {
                let mut state = admin_state_for_token.write();
                state.signup_feedback = Some(ActionFeedback::Error(
                    "Provide the admin password to generate a signup token.".into(),
                ));
                return;
            }

            {
                let mut state = admin_state_for_token.write();
                state.signup_in_flight = true;
                state.signup_feedback = Some(ActionFeedback::Info(
                    "Requesting a new signup token…".into(),
                ));
                state.signup_token = None;
            }

            let mut admin_state_task = admin_state_for_token.clone();
            spawn(async move {
                let result = admin::generate_signup_token(&admin_url, &password).await;
                let mut state = admin_state_task.write();
                match result {
                    Ok(token) => {
                        state.signup_in_flight = false;
                        state.signup_token = Some(token);
                        state.signup_feedback =
                            Some(ActionFeedback::Success("Generated a signup token.".into()));
                        state.bump_info_refresh();
                    }
                    Err(err) => {
                        state.signup_in_flight = false;
                        state.signup_feedback = Some(ActionFeedback::Error(format!(
                            "Failed to generate token: {}",
                            err
                        )));
                    }
                }
            });
        } else {
            let mut state = admin_state_for_token.write();
            state.signup_feedback = Some(ActionFeedback::Error(
                "Start the homeserver to create signup tokens.".into(),
            ));
        }
    };

    let status_for_delete = status.clone();
    let mut admin_state_for_delete = admin_state.clone();
    let on_delete_entry = move |_| {
        let status_snapshot = status_for_delete.read().clone();
        if let ServerStatus::Running(info) = status_snapshot {
            let admin_url = info.admin_url.clone();
            let (password, pubkey, entry_path) = {
                let state = admin_state_for_delete.read();
                (
                    state.password.clone(),
                    state.delete_form.pubkey.clone(),
                    state.delete_form.entry_path.clone(),
                )
            };

            if password.trim().is_empty() {
                let mut state = admin_state_for_delete.write();
                state.delete_form.feedback = Some(ActionFeedback::Error(
                    "Provide the admin password to delete an entry.".into(),
                ));
                return;
            }

            let target = match sanitize_entry_target(&pubkey, &entry_path) {
                Ok(target) => target,
                Err(message) => {
                    let mut state = admin_state_for_delete.write();
                    state.delete_form.feedback = Some(ActionFeedback::Error(message));
                    return;
                }
            };

            {
                let mut state = admin_state_for_delete.write();
                state.delete_form.in_flight = true;
                state.delete_form.feedback = Some(ActionFeedback::Info("Deleting entry…".into()));
            }

            let mut admin_state_task = admin_state_for_delete.clone();
            spawn(async move {
                let result = admin::delete_entry(&admin_url, &password, &target).await;
                let mut state = admin_state_task.write();
                state.delete_form.in_flight = false;
                match result {
                    Ok(()) => {
                        state.delete_form.feedback =
                            Some(ActionFeedback::Success("Entry deleted.".into()));
                        state.bump_info_refresh();
                    }
                    Err(err) => {
                        state.delete_form.feedback = Some(ActionFeedback::Error(format!(
                            "Failed to delete entry: {}",
                            err
                        )));
                    }
                }
            });
        } else {
            let mut state = admin_state_for_delete.write();
            state.delete_form.feedback = Some(ActionFeedback::Error(
                "Start the homeserver to delete entries.".into(),
            ));
        }
    };

    let on_disable_user = {
        let status = status.clone();
        let admin_state = admin_state.clone();
        move |_| toggle_user_access(status.clone(), admin_state.clone(), true)
    };
    let on_enable_user = {
        let status = status.clone();
        let admin_state = admin_state.clone();
        move |_| toggle_user_access(status.clone(), admin_state.clone(), false)
    };

    let mut admin_state_for_delete_pubkey = admin_state.clone();
    let mut admin_state_for_delete_path = admin_state.clone();
    let mut admin_state_for_disable_pubkey = admin_state.clone();

    rsx! {
        section { class: "admin-panel",
            div { class: "admin-panel-header",
                div { class: "admin-panel-heading",
                    h2 { "Admin tools" }
                    p { "Monitor your homeserver and perform maintenance tasks while it's running." }
                }
                div { class: "admin-panel-buttons",
                    button { class: "secondary", onclick: on_refresh_info, "Refresh stats" }
                }
            }
            div { class: "admin-card admin-stats-card",
                h3 { "Homeserver stats" }
                {info_section}
            }
            div { class: "admin-actions-grid",
                div { class: "admin-card",
                    h3 { "Credentials & tokens" }
                    p { "Use your admin password to authenticate API requests." }
                    label { "Admin password" }
                    input {
                        r#type: "password",
                        value: "{admin_snapshot.password}",
                        oninput: on_password_change,
                        placeholder: "Configured in config.toml",
                    }
                    div { class: "button-row",
                        button { class: "secondary", onclick: on_use_config_password, "Use config value" }
                        button { class: "action", onclick: on_generate_token, disabled: admin_snapshot.signup_in_flight, "Gen signup token" }
                    }
                    if let Some(feedback) = admin_snapshot.signup_feedback.clone() {
                        div { class: "admin-feedback {feedback.class()}", "{feedback.message()}" }
                    }
                    if let Some(token) = admin_snapshot.signup_token.clone() {
                        pre { class: "token-display", "{token}" }
                    }
                }
                div { class: "admin-card",
                    h3 { "Delete entry" }
                    p { "Remove a file or directory stored under a user's /pub drive." }
                    label { "Tenant pubkey" }
                    input {
                        r#type: "text",
                        value: "{admin_snapshot.delete_form.pubkey}",
                        oninput: move |evt: FormEvent| {
                            let mut state = admin_state_for_delete_pubkey.write();
                            state.delete_form.pubkey = evt.value();
                        },
                        placeholder: "pk...",
                    }
                    label { "Entry path" }
                    input {
                        r#type: "text",
                        value: "{admin_snapshot.delete_form.entry_path}",
                        oninput: move |evt: FormEvent| {
                            let mut state = admin_state_for_delete_path.write();
                            state.delete_form.entry_path = evt.value();
                        },
                        placeholder: "/pub/path/to/file.txt",
                    }
                    div { class: "button-row",
                        button {
                            class: "action",
                            onclick: on_delete_entry,
                            disabled: admin_snapshot.delete_form.in_flight,
                            "Delete entry"
                        }
                    }
                    if let Some(feedback) = admin_snapshot.delete_form.feedback.clone() {
                        div { class: "admin-feedback {feedback.class()}", "{feedback.message()}" }
                    }
                }
                div { class: "admin-card",
                    h3 { "User access" }
                    p { "Disable or enable a user's homeserver access." }
                    label { "Tenant pubkey" }
                    input {
                        r#type: "text",
                        value: "{admin_snapshot.disable_form.pubkey}",
                        oninput: move |evt: FormEvent| {
                            let mut state = admin_state_for_disable_pubkey.write();
                            state.disable_form.pubkey = evt.value();
                        },
                        placeholder: "pk...",
                    }
                    div { class: "button-row",
                        button {
                            class: "secondary",
                            onclick: on_disable_user,
                            disabled: admin_snapshot.disable_form.in_flight,
                            "Disable user"
                        }
                        button {
                            class: "secondary",
                            onclick: on_enable_user,
                            disabled: admin_snapshot.disable_form.in_flight,
                            "Enable user"
                        }
                    }
                    if let Some(feedback) = admin_snapshot.disable_form.feedback.clone() {
                        div { class: "admin-feedback {feedback.class()}", "{feedback.message()}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Hero() -> Element {
    rsx! {
        div { class: "hero",
            img {
                src: LazyLock::force(&LOGO_DATA_URI).as_str(),
                alt: "Pubky logo",
            }
            div { class: "hero-content",
                h1 { "Homeserver" }
                p {
                    "Start a Pubky homeserver with a single click. Configure endpoints, save the settings, and keep this window open while your node is online."
                }
            }
        }
    }
}

#[component]
fn ControlsPanel(
    data_dir: Signal<String, SyncStorage>,
    network: Signal<NetworkProfile, SyncStorage>,
    config_state: Signal<ConfigState, SyncStorage>,
    status: Signal<ServerStatus, SyncStorage>,
    running_server: Signal<Option<RunningServer>, SyncStorage>,
) -> Element {
    let status_snapshot = status.read().clone();
    let start_disabled = matches!(
        status_snapshot,
        ServerStatus::Starting | ServerStatus::Running(_) | ServerStatus::Stopping
    );
    let stop_disabled = matches!(
        status_snapshot,
        ServerStatus::Idle | ServerStatus::Starting | ServerStatus::Stopping
    );
    let restart_blocked = matches!(
        status_snapshot,
        ServerStatus::Starting | ServerStatus::Stopping
    );

    let selected_network = *network.read();
    let network_for_start = network;
    let data_dir_for_start = data_dir;
    let mut status_for_start = status;
    let mut running_for_start = running_server;
    let status_for_stop = status;
    let running_for_stop = running_server;
    let mut config_state_for_reload = config_state;
    let data_dir_for_reload = data_dir;
    let mut config_state_for_save = config_state;
    let data_dir_for_save = data_dir;
    let status_for_save = status;
    let running_for_save = running_server;
    let network_for_save = network;

    rsx! {
        section { class: "controls",
            NetworkSelector { selected: selected_network, on_select: move |profile| *network.write() = profile }
            DataDirInput { value: data_dir.read().clone(), on_change: move |value| *data_dir.write() = value }
            ConfigEditor {
                config_state,
                restart_blocked,
                on_reload: move |_| {
                    let dir = data_dir_for_reload.read().to_string();
                    match load_config_form_from_dir(&dir) {
                        Ok(form) => {
                            let mut state = config_state_for_reload.write();
                            state.form = form;
                            state.dirty = false;
                            state.feedback = None;
                        }
                        Err(err) => {
                            let mut state = config_state_for_reload.write();
                            state.feedback = Some(ConfigFeedback::PersistenceError(err.to_string()));
                        }
                    }
                },
                on_save_and_restart: move |_| {
                    let form_snapshot = {
                        let state = config_state_for_save.read();
                        state.form.clone()
                    };
                    let dir = data_dir_for_save.read().to_string();

                    match persist_config_form(&dir, &form_snapshot) {
                        Ok(_outcome) => {
                            let selection = *network_for_save.read();
                            let start_spec = match resolve_start_spec(selection, &dir) {
                                Ok(spec) => spec,
                                Err(err) => {
                                    let mut state = config_state_for_save.write();
                                    state.feedback = Some(ConfigFeedback::ValidationError(err.to_string()));
                                    return;
                                }
                            };

                            {
                                let mut state = config_state_for_save.write();
                                state.dirty = false;
                                state.feedback = Some(ConfigFeedback::Saved);
                            }

                            stop_current_server(
                                status_for_save,
                                running_for_save,
                                Some(move || {
                                    spawn_start_task(
                                        start_spec,
                                        status_for_save,
                                        running_for_save,
                                    );
                                }),
                            );
                        }
                        Err(err) => {
                            let mut state = config_state_for_save.write();
                            state.feedback = Some(ConfigFeedback::PersistenceError(err.to_string()));
                        }
                    }
                }
            }
            ActionButtons {
                start_disabled,
                stop_disabled,
                on_start: move |_| {
                    let selection = *network_for_start.read();
                    let data_dir_value = data_dir_for_start.read().to_string();
                    let start_spec = match resolve_start_spec(selection, &data_dir_value) {
                        Ok(spec) => spec,
                        Err(err) => {
                            *status_for_start.write() = ServerStatus::Error(err.to_string());
                            return;
                        }
                    };

                    running_for_start.write().take();
                    spawn_start_task(start_spec, status_for_start, running_for_start);
                },
                on_stop: move |_| {
                    stop_current_server(status_for_stop, running_for_stop, None::<fn()>);
                }
            }
        }
    }
}

#[component]
fn NetworkSelector(selected: NetworkProfile, on_select: EventHandler<NetworkProfile>) -> Element {
    rsx! {
        div { class: "network-selector",
            label { "Select network" }
            div { class: "network-options",
                label { class: "network-option",
                    input {
                        r#type: "radio",
                        name: "network",
                        value: "mainnet",
                        checked: matches!(selected, NetworkProfile::Mainnet),
                        onchange: move |_| on_select.call(NetworkProfile::Mainnet),
                    }
                    span { "Mainnet" }
                }
                label { class: "network-option",
                    input {
                        r#type: "radio",
                        name: "network",
                        value: "testnet",
                        checked: matches!(selected, NetworkProfile::Testnet),
                        onchange: move |_| on_select.call(NetworkProfile::Testnet),
                    }
                    span { "Static Testnet" }
                }
            }
            p { class: "footnote",
                "Testnet runs a local DHT, relays, and homeserver with fixed ports using pubky-testnet."
            }
        }
    }
}

#[component]
fn DataDirInput(value: String, on_change: EventHandler<String>) -> Element {
    rsx! {
        div {
            label { r#"Data directory"# }
            div { class: "data-dir-row",
                input {
                    r#type: "text",
                    value: "{value}",
                    placeholder: r#"~/Library/Application Support/Pubky"#,
                    oninput: move |evt| on_change.call(evt.value()),
                }
            }
            p { class: "footnote",
                "Config, logs, and keys live inside this folder. The homeserver will create missing files automatically."
            }
        }
    }
}

#[component]
fn ConfigEditor(
    config_state: Signal<ConfigState, SyncStorage>,
    restart_blocked: bool,
    on_reload: EventHandler<()>,
    on_save_and_restart: EventHandler<()>,
) -> Element {
    let snapshot = config_state.read().clone();
    let ConfigForm {
        signup_mode,
        drive_pubky_listen_socket,
        drive_icann_listen_socket,
        admin_listen_socket,
        admin_password,
        pkdns_public_ip,
        pkdns_public_pubky_tls_port,
        pkdns_public_icann_http_port,
        pkdns_icann_domain,
        logging_level,
    } = snapshot.form.clone();

    let save_disabled = restart_blocked || !snapshot.dirty;

    let feedback = snapshot.feedback.clone();
    let config_state_pubky = config_state;
    let config_state_icann = config_state;
    let config_state_admin_socket = config_state;
    let config_state_admin_password = config_state;
    let config_state_public_ip = config_state;
    let config_state_tls_port = config_state;
    let config_state_http_port = config_state;
    let config_state_icann_domain = config_state;
    let config_state_logging = config_state;

    rsx! {
        div { class: "config-editor",
            div { class: "config-editor-header",
                label { "Homeserver configuration" }
                button { class: "secondary", onclick: move |_: MouseEvent| on_reload.call(()), "Reload from disk" }
            }

            SignupModePicker { selection: signup_mode, config_state }

            div { class: "config-grid",
                ConfigField {
                    label: "Pubky TLS listen socket",
                    value: drive_pubky_listen_socket,
                    placeholder: "127.0.0.1:6287",
                    on_change: move |value| {
                        modify_config_form(config_state_pubky, |form| {
                            form.drive_pubky_listen_socket = value;
                        });
                    },
                }
                ConfigField {
                    label: "ICANN HTTP listen socket",
                    value: drive_icann_listen_socket,
                    placeholder: "127.0.0.1:6286",
                    on_change: move |value| {
                        modify_config_form(config_state_icann, |form| {
                            form.drive_icann_listen_socket = value;
                        });
                    },
                }
                ConfigField {
                    label: "Admin listen socket",
                    value: admin_listen_socket,
                    placeholder: "127.0.0.1:6288",
                    on_change: move |value| {
                        modify_config_form(config_state_admin_socket, |form| {
                            form.admin_listen_socket = value;
                        });
                    },
                }
                ConfigField {
                    label: "Admin password",
                    value: admin_password,
                    placeholder: "admin",
                    on_change: move |value| {
                        modify_config_form(config_state_admin_password, |form| {
                            form.admin_password = value;
                        });
                    },
                }
                ConfigField {
                    label: "Public IP address",
                    value: pkdns_public_ip,
                    placeholder: "127.0.0.1",
                    on_change: move |value| {
                        modify_config_form(config_state_public_ip, |form| {
                            form.pkdns_public_ip = value;
                        });
                    },
                }
                ConfigField {
                    label: "Public Pubky TLS port",
                    value: pkdns_public_pubky_tls_port,
                    placeholder: "6287",
                    on_change: move |value| {
                        modify_config_form(config_state_tls_port, |form| {
                            form.pkdns_public_pubky_tls_port = value;
                        });
                    },
                }
                ConfigField {
                    label: "Public ICANN HTTP port",
                    value: pkdns_public_icann_http_port,
                    placeholder: "80",
                    on_change: move |value| {
                        modify_config_form(config_state_http_port, |form| {
                            form.pkdns_public_icann_http_port = value;
                        });
                    },
                }
                ConfigField {
                    label: "ICANN domain",
                    value: pkdns_icann_domain,
                    placeholder: "example.com",
                    on_change: move |value| {
                        modify_config_form(config_state_icann_domain, |form| {
                            form.pkdns_icann_domain = value;
                        });
                    },
                }
                ConfigField {
                    label: "Logging level override",
                    value: logging_level,
                    placeholder: "info",
                    on_change: move |value| {
                        modify_config_form(config_state_logging, |form| {
                            form.logging_level = value;
                        });
                    },
                }
            }

            if let Some(feedback) = feedback {
                match feedback {
                    ConfigFeedback::Saved => rsx! {
                        div { class: "config-feedback success",
                            p { "Configuration saved. Restarting homeserver..." }
                        }
                    },
                    ConfigFeedback::ValidationError(message) => rsx! {
                        div { class: "config-feedback error", "{message}" }
                    },
                    ConfigFeedback::PersistenceError(message) => rsx! {
                        div { class: "config-feedback error", "{message}" }
                    },
                }
            }

            div { class: "button-row",
                button {
                    class: "action",
                    disabled: save_disabled,
                    onclick: move |_: MouseEvent| on_save_and_restart.call(()),
                    "Save & Restart"
                }
            }
        }
    }
}

#[component]
fn SignupModePicker(
    config_state: Signal<ConfigState, SyncStorage>,
    selection: SignupMode,
) -> Element {
    rsx! {
        div { class: "signup-mode-group",
            span { "Signup mode" }
            div { class: "signup-mode-options",
                label { class: "signup-mode-option",
                    input {
                        r#type: "radio",
                        name: "signup-mode",
                        value: "token_required",
                        checked: matches!(selection, SignupMode::TokenRequired),
                        onchange: move |_| {
                            modify_config_form(config_state, |form| {
                                form.signup_mode = SignupMode::TokenRequired;
                            });
                        },
                    }
                    span { "Token required" }
                }
                label { class: "signup-mode-option",
                    input {
                        r#type: "radio",
                        name: "signup-mode",
                        value: "open",
                        checked: matches!(selection, SignupMode::Open),
                        onchange: move |_| {
                            modify_config_form(config_state, |form| {
                                form.signup_mode = SignupMode::Open;
                            });
                        },
                    }
                    span { "Open signup" }
                }
            }
        }
    }
}

#[component]
fn ConfigField(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "config-field",
            label { "{label}" }
            input {
                r#type: "text",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |evt: FormEvent| on_change.call(evt.value()),
            }
        }
    }
}

#[component]
fn ActionButtons(
    start_disabled: bool,
    stop_disabled: bool,
    on_start: EventHandler<()>,
    on_stop: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "button-row",
            button {
                class: "action",
                disabled: start_disabled,
                onclick: move |_: MouseEvent| on_start.call(()),
                "Start server"
            }
            button {
                class: "action",
                disabled: stop_disabled,
                onclick: move |_: MouseEvent| on_stop.call(()),
                "Stop server"
            }
        }
    }
}

#[component]
fn FooterNotes(data_dir: String) -> Element {
    rsx! {
        div { class: "footnote",
            "Tip: keep this window open while the homeserver is running. Close it to gracefully stop Pubky."
        }
        div { class: "footnote",
            "Power users can tweak advanced settings in ",
            code { "{data_dir}/config.toml" },
            "."
        }
    }
}

#[component]
fn StatusPanel(status: ServerStatus) -> Element {
    let StatusCopy {
        class_name,
        heading,
        summary,
    } = status_copy(&status);

    let details_section: Option<Element> = match status_details(&status) {
        StatusDetails::Running {
            network_label,
            network_hint,
            admin_url,
            icann_url,
            pubky_url,
            public_key,
        } => Some(rsx! {
            div { class: "status-details",
                p {
                    strong { "Network:" }
                    " {network_label}"
                }
                if let Some(hint) = network_hint {
                    p { "{hint}" }
                }
                p { "Share these endpoints or bookmark them for later:" }
                ul {
                    li {
                        strong { "Admin API:" }
                        " "
                        a { href: "{admin_url}", target: "_blank", rel: "noreferrer", "{admin_url}" }
                    }
                    li {
                        strong { "ICANN HTTP:" }
                        " "
                        a { href: "{icann_url}", target: "_blank", rel: "noreferrer", "{icann_url}" }
                    }
                    li {
                        strong { "Pubky TLS:" }
                        " "
                        a { href: "{pubky_url}", target: "_blank", rel: "noreferrer", "{pubky_url}" }
                    }
                }
                p { "Public key:" }
                pre { class: "public-key", "{public_key}" }
                p { "Anyone can reach your agent with the public key above." }
            }
        }),
        StatusDetails::Error { message } => Some(rsx! {
            div { class: "status-details",
                p { "Check that the directory is writable and the config is valid." }
                pre { class: "public-key", "{message}" }
            }
        }),
        StatusDetails::Message(copy) => Some(rsx! {
            div { class: "status-details",
                p { "{copy}" }
            }
        }),
        StatusDetails::None => None,
    };

    let details_section = details_section.unwrap_or_else(|| rsx! { Fragment {} });

    rsx! {
        div { class: "status-card {class_name}",
            h2 { "{heading}" }
            p { "{summary}" }
            {details_section}
        }
    }
}
