use std::sync::LazyLock;

use dioxus::prelude::*;
use pubky_homeserver::SignupMode;

use super::config::{
    ConfigFeedback, ConfigForm, config_state_from_dir, default_data_dir, load_config_form_from_dir,
    modify_config_form, persist_config_form,
};
use super::state::{NetworkProfile, RunningServer, ServerStatus, resolve_start_spec};
use super::status::{StatusCopy, StatusDetails, status_copy, status_details};
use super::style::{LOGO_DATA_URI, STYLE};
use super::tasks::{spawn_start_task, stop_current_server};

#[component]
pub(crate) fn App() -> Element {
    let initial_data_dir = default_data_dir();
    let initial_config_state = config_state_from_dir(&initial_data_dir);

    let mut data_dir = use_signal_sync(|| initial_data_dir.clone());
    let status = use_signal_sync(ServerStatus::default);
    let suite_handle = use_signal_sync(|| Option::<RunningServer>::None);
    let mut network = use_signal_sync(|| NetworkProfile::Mainnet);
    let config_state = use_signal_sync(|| initial_config_state.clone());

    let start_disabled = matches!(
        *status.peek(),
        ServerStatus::Starting | ServerStatus::Running(_) | ServerStatus::Stopping
    );
    let stop_disabled = matches!(
        *status.peek(),
        ServerStatus::Idle | ServerStatus::Starting | ServerStatus::Stopping
    );
    let restart_blocked = matches!(
        *status.peek(),
        ServerStatus::Starting | ServerStatus::Stopping
    );

    let start_server = {
        let data_dir_signal = data_dir.clone();
        let mut status_signal = status.clone();
        let mut suite_signal = suite_handle.clone();
        let network_signal = network.clone();

        move |_| {
            let selection = *network_signal.read();
            let data_dir_value = data_dir_signal.read().to_string();
            let start_spec = match resolve_start_spec(selection, &data_dir_value) {
                Ok(spec) => spec,
                Err(err) => {
                    *status_signal.write() = ServerStatus::Error(err.to_string());
                    return;
                }
            };

            suite_signal.write().take();
            spawn_start_task(start_spec, status_signal.clone(), suite_signal.clone());
        }
    };

    let stop_server = {
        let status_signal = status.clone();
        let suite_signal = suite_handle.clone();

        move |_| {
            stop_current_server(status_signal.clone(), suite_signal.clone());
        }
    };

    let load_config = {
        let data_dir_signal = data_dir.clone();
        let mut config_signal = config_state.clone();

        move |_| {
            let dir = data_dir_signal.read().to_string();
            match load_config_form_from_dir(&dir) {
                Ok(form) => {
                    let mut state = config_signal.write();
                    state.form = form;
                    state.dirty = false;
                    state.feedback = None;
                }
                Err(err) => {
                    let mut state = config_signal.write();
                    state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                }
            }
        }
    };

    let save_and_restart = {
        let mut config_signal = config_state.clone();
        let data_dir_signal = data_dir.clone();
        let status_signal = status.clone();
        let suite_signal = suite_handle.clone();
        let network_signal = network.clone();

        move |_| {
            let form_snapshot = {
                let state = config_signal.read();
                state.form.clone()
            };
            let dir = data_dir_signal.read().to_string();

            match persist_config_form(&dir, &form_snapshot) {
                Ok(_) => {
                    let selection = *network_signal.read();
                    let start_spec = match resolve_start_spec(selection, &dir) {
                        Ok(spec) => spec,
                        Err(err) => {
                            let mut state = config_signal.write();
                            state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                            return;
                        }
                    };

                    {
                        let mut state = config_signal.write();
                        state.dirty = false;
                        state.feedback = Some(ConfigFeedback::Saved);
                    }

                    stop_current_server(status_signal.clone(), suite_signal.clone());
                    spawn_start_task(start_spec, status_signal.clone(), suite_signal.clone());
                }
                Err(err) => {
                    let mut state = config_signal.write();
                    state.feedback = Some(ConfigFeedback::Error(err.to_string()));
                }
            }
        }
    };

    let data_dir_value = data_dir.read().to_string();
    let status_snapshot = status.read().clone();
    let selected_network = *network.read();
    let config_state_snapshot = config_state.read().clone();
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
    } = config_state_snapshot.form.clone();
    let config_feedback = config_state_snapshot.feedback.clone();
    let save_disabled = restart_blocked || !config_state_snapshot.dirty;

    let config_state_signup_token = config_state.clone();
    let config_state_signup_open = config_state.clone();
    let config_state_pubky = config_state.clone();
    let config_state_icann = config_state.clone();
    let config_state_admin_socket = config_state.clone();
    let config_state_admin_password = config_state.clone();
    let config_state_public_ip = config_state.clone();
    let config_state_tls_port = config_state.clone();
    let config_state_http_port = config_state.clone();
    let config_state_icann_domain = config_state.clone();
    let config_state_logging = config_state.clone();

    rsx! {
        style { "{STYLE}" }
        main { class: "app",
            div { class: "hero",
                img {
                    src: LazyLock::force(&LOGO_DATA_URI).as_str(),
                    alt: "Pubky logo",
                }
                div { class: "hero-content",
                    h1 { "Portable Pubky Homeserver" }
                    p { "It's your data, bring it with you." }
                }
            }

            section { class: "controls",
                div { class: "network-selector",
                    label { "Select network" }
                    div { class: "network-options",
                        label { class: "network-option",
                            input {
                                r#type: "radio",
                                name: "network",
                                value: "mainnet",
                                checked: matches!(selected_network, NetworkProfile::Mainnet),
                                onchange: move |_| {
                                    *network.write() = NetworkProfile::Mainnet;
                                },
                            }
                            span { "Mainnet" }
                        }
                        label { class: "network-option",
                            input {
                                r#type: "radio",
                                name: "network",
                                value: "testnet",
                                checked: matches!(selected_network, NetworkProfile::Testnet),
                                onchange: move |_| {
                                    *network.write() = NetworkProfile::Testnet;
                                },
                            }
                            span { "Static Testnet" }
                        }
                    }
                    p { class: "footnote",
                        "Testnet runs a local DHT, relays, and homeserver with fixed ports using pubky-testnet."
                    }
                }

                div {
                    label { r#"Data directory"# }
                    div { class: "data-dir-row",
                        input {
                            r#type: "text",
                            value: "{data_dir_value}",
                            placeholder: r#"~/Library/Application Support/Pubky"#,
                            oninput: move |evt| {
                                let value = evt.value();
                                *data_dir.write() = value;
                            }
                        }
                    }
                    p { class: "footnote",
                        "Config, logs, and keys live inside this folder. The homeserver will create missing files automatically."
                    }
                }

                div { class: "config-editor",
                    div { class: "config-editor-header",
                        label { "Homeserver configuration" }
                        button { class: "secondary", onclick: load_config, "Reload from disk" }
                    }

                    div { class: "signup-mode-group",
                        span { "Signup mode" }
                        div { class: "signup-mode-options",
                            label { class: "signup-mode-option",
                                input {
                                    r#type: "radio",
                                    name: "signup-mode",
                                    value: "token_required",
                                    checked: matches!(signup_mode, SignupMode::TokenRequired),
                                    onchange: move |_| {
                                        modify_config_form(config_state_signup_token.clone(), |form| {
                                            form.signup_mode = SignupMode::TokenRequired;
                                        });
                                    }
                                }
                                span { "Token required" }
                            }
                            label { class: "signup-mode-option",
                                input {
                                    r#type: "radio",
                                    name: "signup-mode",
                                    value: "open",
                                    checked: matches!(signup_mode, SignupMode::Open),
                                    onchange: move |_| {
                                        modify_config_form(config_state_signup_open.clone(), |form| {
                                            form.signup_mode = SignupMode::Open;
                                        });
                                    }
                                }
                                span { "Open signup" }
                            }
                        }
                    }

                    div { class: "config-grid",
                        div { class: "config-field",
                            label { "Pubky TLS listen socket" }
                            input {
                                r#type: "text",
                                value: "{drive_pubky_listen_socket}",
                                placeholder: "127.0.0.1:6287",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_pubky.clone(), |form| {
                                        form.drive_pubky_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "ICANN HTTP listen socket" }
                            input {
                                r#type: "text",
                                value: "{drive_icann_listen_socket}",
                                placeholder: "127.0.0.1:6286",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_icann.clone(), |form| {
                                        form.drive_icann_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Admin listen socket" }
                            input {
                                r#type: "text",
                                value: "{admin_listen_socket}",
                                placeholder: "127.0.0.1:6288",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_admin_socket.clone(), |form| {
                                        form.admin_listen_socket = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Admin password" }
                            input {
                                r#type: "text",
                                value: "{admin_password}",
                                placeholder: "admin",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_admin_password.clone(), |form| {
                                        form.admin_password = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public IP address" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_ip}",
                                placeholder: "127.0.0.1",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_public_ip.clone(), |form| {
                                        form.pkdns_public_ip = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public Pubky TLS port" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_pubky_tls_port}",
                                placeholder: "6287",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_tls_port.clone(), |form| {
                                        form.pkdns_public_pubky_tls_port = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Public ICANN HTTP port" }
                            input {
                                r#type: "text",
                                value: "{pkdns_public_icann_http_port}",
                                placeholder: "80",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_http_port.clone(), |form| {
                                        form.pkdns_public_icann_http_port = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "ICANN domain" }
                            input {
                                r#type: "text",
                                value: "{pkdns_icann_domain}",
                                placeholder: "example.com",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_icann_domain.clone(), |form| {
                                        form.pkdns_icann_domain = value;
                                    });
                                }
                            }
                        }
                        div { class: "config-field",
                            label { "Logging level override" }
                            input {
                                r#type: "text",
                                value: "{logging_level}",
                                placeholder: "info",
                                oninput: move |evt| {
                                    let value = evt.value();
                                    modify_config_form(config_state_logging.clone(), |form| {
                                        form.logging_level = value;
                                    });
                                }
                            }
                        }
                    }

                    if let Some(feedback) = config_feedback.clone() {
                        match feedback {
                            ConfigFeedback::Saved => rsx! {
                                div { class: "config-feedback success",
                                    p { "Configuration saved. Restarting homeserver..." }
                                }
                            },
                            ConfigFeedback::Error(message) => rsx! {
                                div { class: "config-feedback error", "{message}" }
                            },
                        }
                    }

                    div { class: "button-row",
                        button {
                            class: "action",
                            disabled: save_disabled,
                            onclick: save_and_restart,
                            "Save & Restart"
                        }
                    }
                }

                div { class: "button-row",
                    button {
                        class: "action",
                        disabled: start_disabled,
                        onclick: start_server,
                        "Start server"
                    }
                    button {
                        class: "action",
                        disabled: stop_disabled,
                        onclick: stop_server,
                        "Stop server"
                    }
                }
            }

            StatusPanel { status: status_snapshot.clone() }

            div { class: "footnote",
                "Tip: keep this window open while the homeserver is running. Close it to gracefully stop Pubky." }
            div { class: "footnote",
                "Power users can tweak advanced settings in ",
                code { "{data_dir_value}/config.toml" },
                "."
            }
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
