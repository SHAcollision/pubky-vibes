use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use pubky::Keypair;
use std::path::PathBuf;

use crate::tabs::KeysTabState;
use crate::utils::file_dialog::{self, FileDialogResult};
use crate::utils::logging::ActivityLog;
use crate::utils::mobile::{is_android_touch, touch_copy, touch_tooltip};
use crate::utils::recovery::{
    decode_secret_key, load_keypair_from_recovery, normalize_pkarr_path,
    save_keypair_to_recovery_file,
};

pub fn render_keys_tab(state: KeysTabState, logs: ActivityLog) -> Element {
    let KeysTabState {
        keypair,
        secret_input,
        recovery_path,
        recovery_passphrase,
    } = state;
    let current_public = {
        let guard = keypair.read();
        guard
            .as_ref()
            .map(|kp| kp.public_key().to_string())
            .unwrap_or_else(|| "–".to_string())
    };
    let public_copy_value = if current_public != "–" {
        Some(current_public.clone())
    } else {
        None
    };
    let public_copy_success = if is_android_touch() {
        Some(String::from("Copied public key to clipboard"))
    } else {
        None
    };
    let secret_value = { secret_input.read().clone() };
    let recovery_path_value = { recovery_path.read().clone() };
    let recovery_pass_value = { recovery_passphrase.read().clone() };

    let mut generate_secret_input = secret_input;
    let mut generate_keypair = keypair;
    let generate_logs = logs.clone();

    let mut export_secret_input = secret_input;
    let export_keypair = keypair;
    let export_logs = logs.clone();

    let mut import_keypair_signal = keypair;
    let import_secret_signal = secret_input;
    let import_logs = logs.clone();

    let load_path_signal = recovery_path;
    let load_pass_signal = recovery_passphrase;
    let load_keypair_signal = keypair;
    let load_secret_signal = secret_input;
    let load_logs = logs.clone();

    let save_path_signal = recovery_path;
    let save_pass_signal = recovery_passphrase;
    let save_keypair_signal = keypair;
    let save_logs = logs.clone();

    let mut secret_input_binding = secret_input;
    let mut recovery_pass_binding = recovery_passphrase;
    let mut choose_recovery_path_signal = recovery_path;
    let mut recovery_path_binding = recovery_path;
    let choose_logs = logs;

    rsx! {
        div { class: "tab-body tight",
            section { class: "card",
                h2 { "Key material" }
                p { class: "helper-text",
                    "Generate or import keys."
                    if let Some(value) = public_copy_value.clone() {
                        span {
                            class: "copyable-inline",
                            title: "Tap to copy the current public key",
                            "data-touch-tooltip": touch_tooltip(
                                "Tap to copy the current public key",
                            ),
                            "data-touch-copy": touch_copy(value.clone()),
                            "data-copy-success": public_copy_success.clone(),
                            " Current public key: ",
                            span { class: "mono", {value.clone()} }
                        }
                        "."
                    } else {
                        { format!(" Current public key: {}.", current_public) }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Generate a brand-new Ed25519 signing key and load it here",
                        "data-touch-tooltip": touch_tooltip(
                            "Generate a brand-new Ed25519 signing key and load it here",
                        ),
                        onclick: move |_| {
                            let kp = Keypair::random();
                            generate_secret_input.set(STANDARD.encode(kp.secret_key()));
                            generate_keypair.set(Some(kp.clone()));
                            generate_logs.success(format!("Generated signer {}", kp.public_key()));
                        },
                        "Generate random key"
                    }
                    button {
                        class: "action secondary",
                        title: "Copy the active signer secret (as base64) into the editor without touching disk",
                        "data-touch-tooltip": touch_tooltip(
                            "Copy the active signer secret (as base64) into the editor without touching disk",
                        ),
                        onclick: move |_| {
                            if let Some(kp) = export_keypair.read().as_ref() {
                                export_secret_input.set(STANDARD.encode(kp.secret_key()));
                                export_logs.info("Secret key exported to editor");
                            } else {
                                export_logs.error("No key loaded");
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
                            title: "Paste or edit the base64-encoded 32-byte secret for your signing key",
                            "data-touch-tooltip": touch_tooltip(
                                "Paste or edit the base64-encoded 32-byte secret for your signing key",
                            ),
                            placeholder: "Base64 encoded 32-byte secret key",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Activate the signer using the secret from the editor",
                        "data-touch-tooltip": touch_tooltip(
                            "Activate the signer using the secret from the editor",
                        ),
                        onclick: move |_| {
                            let secret = import_secret_signal.read().clone();
                            match decode_secret_key(&secret) {
                                Ok(kp) => {
                                    import_keypair_signal.set(Some(kp.clone()));
                                    import_logs.success(format!("Loaded key for {}", kp.public_key()));
                                }
                                Err(err) => import_logs.error(format!("Invalid secret key: {err}")),
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
                            input {
                                class: "file-path-display",
                                value: recovery_path_value.clone(),
                                oninput: move |evt| recovery_path_binding.set(evt.value()),
                                placeholder: "Enter recovery file path",
                            }
                            button {
                                class: "action secondary",
                                title: "Browse for an existing PKARR or Pubky recovery file to import",
                                "data-touch-tooltip": touch_tooltip(
                                    "Browse for an existing PKARR or Pubky recovery file to import",
                                ),
                                onclick: move |_| {
                                    match file_dialog::pick_file() {
                                        FileDialogResult::Selected(path) => {
                                            choose_recovery_path_signal.set(path.display().to_string());
                                        }
                                        FileDialogResult::Unavailable => {
                                            choose_logs.info(file_dialog::MANUAL_ENTRY_HINT)
                                        }
                                        FileDialogResult::Cancelled => {}
                                    }
                                },
                                "Choose file"
                            }
                        }
                    }
                    label {
                        "Passphrase"
                        input {
                            r#type: "password",
                            value: recovery_pass_value.clone(),
                            oninput: move |evt| recovery_pass_binding.set(evt.value()),
                            title: "Passphrase used to decrypt PKARR recovery bundles",
                            "data-touch-tooltip": touch_tooltip(
                                "Passphrase used to decrypt PKARR recovery bundles",
                            ),
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Open and decrypt a PKARR recovery file and load its key into the tool",
                        "data-touch-tooltip": touch_tooltip(
                            "Open and decrypt a PKARR recovery file and load its key into the tool",
                        ),
                        onclick: move |_| {
                            let raw_path = load_path_signal.read().clone();
                            let passphrase = load_pass_signal.read().clone();
                            let mut immediate_path_signal = load_path_signal;
                            let chosen_path = if raw_path.trim().is_empty() {
                                match file_dialog::pick_file() {
                                    FileDialogResult::Selected(path) => {
                                        let display = path.display().to_string();
                                        immediate_path_signal.set(display.clone());
                                        Some(display)
                                    }
                                    FileDialogResult::Unavailable => {
                                        load_logs.info(file_dialog::MANUAL_ENTRY_HINT);
                                        None
                                    }
                                    FileDialogResult::Cancelled => None,
                                }
                            } else {
                                Some(raw_path.clone())
                            };
                            if let Some(selected_path) = chosen_path {
                                let mut keypair_signal = load_keypair_signal;
                                let mut secret_signal = load_secret_signal;
                                let mut path_signal = load_path_signal;
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
                                            logs_task.success(format!(
                                                "Decrypted recovery file {} for {}",
                                                normalized.display(),
                                                kp.public_key()
                                            ));
                                        }
                                        Err(err) => logs_task.error(format!(
                                            "Failed to load recovery file: {err}"
                                        )),
                                    }
                                });
                            }
                        },
                        "Load from recovery file"
                    }
                    button {
                        class: "action secondary",
                        title: "Encrypt the active keypair into a PKARR-compatible bundle and save it",
                        "data-touch-tooltip": touch_tooltip(
                            "Encrypt the active keypair into a PKARR-compatible bundle and save it",
                        ),
                        onclick: move |_| {
                            if let Some(kp) = save_keypair_signal.read().as_ref().cloned() {
                                let raw_path = save_path_signal.read().clone();
                                let mut immediate_path_signal = save_path_signal;
                                let chosen_path = if raw_path.trim().is_empty() {
                                    match file_dialog::save_file() {
                                        FileDialogResult::Selected(path) => {
                                            let display = path.display().to_string();
                                            immediate_path_signal.set(display.clone());
                                            Some(display)
                                        }
                                        FileDialogResult::Unavailable => {
                                            save_logs.info(file_dialog::MANUAL_ENTRY_HINT);
                                            None
                                        }
                                        FileDialogResult::Cancelled => None,
                                    }
                                } else {
                                    Some(raw_path.clone())
                                };
                                if let Some(selected_path) = chosen_path {
                                    let passphrase = save_pass_signal.read().clone();
                                    let mut path_signal = save_path_signal;
                                    let logs_task = save_logs.clone();
                                    spawn(async move {
                                        match save_keypair_to_recovery_file(&kp, &selected_path, &passphrase) {
                                            Ok(path) => {
                                                path_signal.set(path.display().to_string());
                                                logs_task.success(format!(
                                                    "Recovery file saved to {}",
                                                    path.display()
                                                ));
                                            }
                                            Err(err) => logs_task.error(format!(
                                                "Failed to save recovery file: {err}"
                                            )),
                                        }
                                    });
                                }
                            } else {
                                save_logs.error("Generate or import a key first");
                            }
                        },
                        "Save recovery file"
                    }
                }
            }
        }
    }
}
