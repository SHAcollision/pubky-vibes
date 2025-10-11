use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use pubky::Keypair;
use rfd::FileDialog;
use std::path::PathBuf;

use crate::utils::logging::{LogEntry, LogLevel, push_log};
use crate::utils::recovery::{
    decode_secret_key, load_keypair_from_recovery, normalize_pkarr_path,
    save_keypair_to_recovery_file,
};

pub fn render_keys_tab(
    keypair: Signal<Option<Keypair>>,
    secret_input: Signal<String>,
    recovery_path: Signal<String>,
    recovery_passphrase: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let current_public = {
        let guard = keypair.read();
        guard
            .as_ref()
            .map(|kp| kp.public_key().to_string())
            .unwrap_or_else(|| "–".to_string())
    };
    let secret_value = { secret_input.read().clone() };
    let recovery_path_value = { recovery_path.read().clone() };
    let recovery_pass_value = { recovery_passphrase.read().clone() };
    let recovery_path_display = if recovery_path_value.trim().is_empty() {
        "No file selected".to_string()
    } else {
        recovery_path_value.clone()
    };

    let mut generate_secret_input = secret_input.clone();
    let mut generate_keypair = keypair.clone();
    let generate_logs = logs.clone();

    let mut export_secret_input = secret_input.clone();
    let export_keypair = keypair.clone();
    let export_logs = logs.clone();

    let mut import_keypair_signal = keypair.clone();
    let import_secret_signal = secret_input.clone();
    let import_logs = logs.clone();

    let load_path_signal = recovery_path.clone();
    let load_pass_signal = recovery_passphrase.clone();
    let load_keypair_signal = keypair.clone();
    let load_secret_signal = secret_input.clone();
    let load_logs = logs.clone();

    let save_path_signal = recovery_path.clone();
    let save_pass_signal = recovery_passphrase.clone();
    let save_keypair_signal = keypair.clone();
    let save_logs = logs.clone();

    let mut secret_input_binding = secret_input.clone();
    let mut recovery_pass_binding = recovery_passphrase.clone();
    let mut choose_recovery_path_signal = recovery_path.clone();

    rsx! {
        div { class: "tab-body tight",
            section { class: "card",
                h2 { "Key material" }
                p { class: "helper-text", "Generate or import keys. Current public key: {current_public}." }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let kp = Keypair::random();
                            generate_secret_input.set(STANDARD.encode(kp.secret_key()));
                            generate_keypair.set(Some(kp.clone()));
                            push_log(generate_logs, LogLevel::Success, format!("Generated signer {}", kp.public_key()));
                        },
                        "Generate random key"
                    }
                    button { class: "action secondary", onclick: move |_| {
                            if let Some(kp) = export_keypair.read().as_ref() {
                                export_secret_input.set(STANDARD.encode(kp.secret_key()));
                                push_log(export_logs, LogLevel::Info, "Secret key exported to editor");
                            } else {
                                push_log(export_logs, LogLevel::Error, "No key loaded");
                            }
                        },
                        "Show secret key"
                    }
                }
                div { class: "form-grid",
                    label {
                        "Secret key (base64)"
                        textarea {
                            class: "tall",
                            value: secret_value,
                            oninput: move |evt| secret_input_binding.set(evt.value()),
                            placeholder: "Base64 encoded 32-byte secret key",
                        }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let secret = import_secret_signal.read().clone();
                            match decode_secret_key(&secret) {
                                Ok(kp) => {
                                    import_keypair_signal.set(Some(kp.clone()));
                                    push_log(import_logs.clone(), LogLevel::Success, format!("Loaded key for {}", kp.public_key()));
                                }
                                Err(err) => push_log(import_logs, LogLevel::Error, format!("Invalid secret key: {err}")),
                            }
                        },
                        "Import secret"
                    }
                }
            }
            section { class: "card",
                h2 { "Recovery files" }
                div { class: "form-grid",
                    label {
                        "Recovery file path"
                        div { class: "file-picker-row",
                            span { class: "file-path-display", "{recovery_path_display}" }
                            button {
                                class: "action secondary",
                                onclick: move |_| {
                                    if let Some(path) = FileDialog::new().pick_file() {
                                        choose_recovery_path_signal.set(path.display().to_string());
                                    }
                                },
                                "Choose file"
                            }
                        }
                    }
                    label {
                        "Passphrase"
                        input { r#type: "password", value: recovery_pass_value.clone(), oninput: move |evt| recovery_pass_binding.set(evt.value()) }
                    }
                }
                div { class: "small-buttons",
                    button { class: "action", onclick: move |_| {
                            let raw_path = load_path_signal.read().clone();
                            let passphrase = load_pass_signal.read().clone();
                            let mut immediate_path_signal = load_path_signal.clone();
                            let chosen_path = if raw_path.trim().is_empty() {
                                FileDialog::new().pick_file().map(|path| {
                                    let display = path.display().to_string();
                                    immediate_path_signal.set(display.clone());
                                    display
                                })
                            } else {
                                Some(raw_path.clone())
                            };
                            if let Some(selected_path) = chosen_path {
                                let mut keypair_signal = load_keypair_signal.clone();
                                let mut secret_signal = load_secret_signal.clone();
                                let mut path_signal = load_path_signal.clone();
                                let logs_task = load_logs.clone();
                                let passphrase_for_task = passphrase.clone();
                                spawn(async move {
                                    let outcome = (|| -> Result<(Keypair, PathBuf)> {
                                        let normalized = normalize_pkarr_path(&selected_path)?;
                                        let kp = load_keypair_from_recovery(&normalized, &passphrase_for_task)?;
                                        Ok((kp, normalized))
                                    })();
                                    match outcome {
                                        Ok((kp, normalized)) => {
                                            secret_signal.set(STANDARD.encode(kp.secret_key()));
                                            keypair_signal.set(Some(kp.clone()));
                                            path_signal.set(normalized.display().to_string());
                                            push_log(
                                                logs_task,
                                                LogLevel::Success,
                                                format!(
                                                    "Decrypted recovery file {} for {}",
                                                    normalized.display(),
                                                    kp.public_key()
                                                ),
                                            );
                                        }
                                        Err(err) => push_log(
                                            logs_task,
                                            LogLevel::Error,
                                            format!("Failed to load recovery file: {err}"),
                                        ),
                                    }
                                });
                            }
                        },
                        "Load from recovery file"
                    }
                    button { class: "action secondary", onclick: move |_| {
                            if let Some(kp) = save_keypair_signal.read().as_ref().cloned() {
                                let raw_path = save_path_signal.read().clone();
                                let mut immediate_path_signal = save_path_signal.clone();
                                let chosen_path = if raw_path.trim().is_empty() {
                                    FileDialog::new().save_file().map(|path| {
                                        let display = path.display().to_string();
                                        immediate_path_signal.set(display.clone());
                                        display
                                    })
                                } else {
                                    Some(raw_path.clone())
                                };
                                if let Some(selected_path) = chosen_path {
                                    let passphrase = save_pass_signal.read().clone();
                                    let mut path_signal = save_path_signal.clone();
                                    let logs_task = save_logs.clone();
                                    spawn(async move {
                                        match save_keypair_to_recovery_file(&kp, &selected_path, &passphrase) {
                                            Ok(path) => {
                                                path_signal.set(path.display().to_string());
                                                push_log(
                                                    logs_task,
                                                    LogLevel::Success,
                                                    format!("Recovery file saved to {}", path.display()),
                                                );
                                            }
                                            Err(err) => push_log(
                                                logs_task,
                                                LogLevel::Error,
                                                format!("Failed to save recovery file: {err}"),
                                            ),
                                        }
                                    });
                                }
                            } else {
                                push_log(save_logs.clone(), LogLevel::Error, "Generate or import a key first");
                            }
                        },
                        "Save recovery file"
                    }
                }
            }
        }
    }
}
