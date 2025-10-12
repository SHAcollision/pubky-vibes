use std::sync::LazyLock;

use dioxus::events::{FormEvent, MouseEvent};
use dioxus::prelude::*;
use dioxus::signals::{Signal, SyncStorage};
use pubky_homeserver::SignupMode;

use super::config::{
    ConfigFeedback, ConfigForm, ConfigState, config_state_from_dir, default_data_dir,
    load_config_form_from_dir, modify_config_form, persist_config_form,
};
use super::state::{NetworkProfile, RunningServer, ServerStatus, resolve_start_spec};
use super::status::{StatusCopy, StatusDetails, status_copy, status_details};
use super::style::{LOGO_DATA_URI, STYLE};
use super::tasks::{spawn_start_task, stop_current_server};

#[component]
pub fn App() -> Element {
    let initial_data_dir = default_data_dir();
    let initial_config_state = config_state_from_dir(&initial_data_dir);

    let data_dir = use_signal_sync(|| initial_data_dir.clone());
    let status = use_signal_sync(ServerStatus::default);
    let running_server = use_signal_sync(|| Option::<RunningServer>::None);
    let network = use_signal_sync(|| NetworkProfile::Mainnet);
    let config_state = use_signal_sync(|| initial_config_state.clone());

    let status_snapshot = status.read().clone();
    let data_dir_snapshot = data_dir.read().clone();

    rsx! {
        style { "{STYLE}" }
        main { class: "app",
            Hero {}
            ControlsPanel {
                data_dir,
                network,
                config_state,
                status,
                running_server
            }
            StatusPanel { status: status_snapshot }
            FooterNotes { data_dir: data_dir_snapshot }
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
