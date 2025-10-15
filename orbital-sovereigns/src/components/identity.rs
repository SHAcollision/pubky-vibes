use dioxus::prelude::WritableExt;
use dioxus::prelude::*;

use crate::models::CommanderIdentity;
use crate::services::{LogEntry, LogLevel, push_log};

#[component]
#[allow(non_snake_case)]
pub fn IdentityPanel(
    mut identity: Signal<CommanderIdentity>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let secret_buffer = use_signal(String::new);
    let label_value = { identity.read().label().to_string() };
    let public_key = identity
        .read()
        .keypair()
        .map(|kp| kp.public_key().to_string())
        .unwrap_or_else(|| "—".to_string());
    let secret_value = { secret_buffer.read().clone() };

    let mut label_binding = identity.clone();
    let mut secret_binding = secret_buffer.clone();

    let mut generate_identity = identity.clone();
    let generate_logs = logs.clone();

    let export_identity = identity.clone();
    let mut export_buffer = secret_buffer.clone();
    let export_logs = logs.clone();

    let import_buffer = secret_buffer.clone();
    let import_logs = logs.clone();

    rsx! {
        div { class: "panel",
            h2 { "Commander" }
            p { "Manage your signing keys. Active public key: {public_key}" }
            label {
                "Callsign"
                input {
                    value: label_value,
                    oninput: move |evt| {
                        label_binding.write().set_label(evt.value());
                    },
                    placeholder: "Captain name",
                }
            }
            div { class: "field-grid",
                label {
                    "Secret key (base64)"
                    textarea {
                        value: secret_value,
                        oninput: move |evt| secret_binding.set(evt.value()),
                        placeholder: "Paste a 32-byte secret key",
                    }
                }
            }
            div { class: "field-grid",
                button {
                    onclick: move |_| {
                        let kp = generate_identity.write().generate();
                        push_log(generate_logs, LogLevel::Success, format!("Generated signer {}", kp.public_key()));
                    },
                    "Generate new key"
                }
                button { class: "secondary",
                    onclick: move |_| {
                        if let Some(secret) = export_identity.read().export_secret_key() {
                            export_buffer.set(secret);
                            push_log(export_logs, LogLevel::Info, "Secret key copied into editor");
                        } else {
                            push_log(export_logs, LogLevel::Warning, "Generate or import a key first");
                        }
                    },
                    "Show secret"
                }
                button { class: "secondary",
                    onclick: move |_| {
                        let value = import_buffer.read().clone();
                        match identity.write().import_secret_key(&value) {
                            Ok(kp) => {
                                push_log(import_logs.clone(), LogLevel::Success, format!("Loaded signer {}", kp.public_key()));
                            }
                            Err(err) => {
                                push_log(import_logs.clone(), LogLevel::Error, format!("Invalid secret key: {err}"));
                            }
                        }
                    },
                    "Import secret"
                }
            }
            if let Some(hint) = identity.read().recovery_hint() {
                p { class: "hint", "Recovery hint: {hint}" }
            }
        }
    }
}
