use dioxus::prelude::*;
use pubky::PublicKey;

use crate::tabs::PkdnsTabState;
use crate::utils::logging::ActivityLog;
use crate::utils::pubky::PubkyFacadeHandle;

#[allow(clippy::clone_on_copy)]
pub fn render_pkdns_tab(
    pubky: PubkyFacadeHandle,
    state: PkdnsTabState,
    logs: ActivityLog,
) -> Element {
    let PkdnsTabState {
        keypair,
        lookup_input,
        lookup_result,
        host_override,
    } = state;

    let lookup_value = { lookup_input.read().clone() };
    let lookup_result_value = { lookup_result.read().clone() };
    let host_override_value = { host_override.read().clone() };

    let mut lookup_binding = lookup_input.clone();
    let mut override_binding = host_override.clone();

    let lookup_logs = logs.clone();
    let lookup_pubky = pubky.clone();
    let lookup_result_signal = lookup_result.clone();

    let self_lookup_logs = logs.clone();
    let self_lookup_pubky = pubky.clone();
    let self_lookup_result_signal = lookup_result.clone();
    let self_lookup_keypair = keypair.clone();

    let publish_if_stale_logs = logs.clone();
    let publish_if_stale_pubky = pubky.clone();
    let publish_if_stale_keypair = keypair.clone();
    let publish_if_stale_override = host_override.clone();
    let publish_if_stale_result_signal = lookup_result.clone();

    let publish_force_logs = logs.clone();
    let publish_force_pubky = pubky.clone();
    let publish_force_keypair = keypair.clone();
    let publish_force_override = host_override.clone();
    let publish_force_result_signal = lookup_result.clone();

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Homeserver lookups" }
                p { class: "helper-text", "Resolve `_pubky` records from PKARR for any user or for the active key." }
                div { class: "form-grid",
                    label {
                        "User public key"
                        input {
                            value: lookup_value,
                            oninput: move |evt| lookup_binding.set(evt.value()),
                            title: "Enter a user's public key to resolve their homeserver via PKDNS",
                            placeholder: "Base32 public key",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Resolve the homeserver registered for this user via PKARR",
                        onclick: move |_| {
                            let query = lookup_input.read().clone();
                            let trimmed = query.trim().to_string();
                            if trimmed.is_empty() {
                                lookup_logs.error("User public key is required");
                                return;
                            }
                            let target_pk = match PublicKey::try_from(trimmed.as_str()) {
                                Ok(pk) => pk,
                                Err(err) => {
                                    lookup_logs.error(format!("Invalid public key: {err}"));
                                    return;
                                }
                            };
                            let Some(pubky_arc) = lookup_pubky.ready_or_log(&lookup_logs) else {
                                return;
                            };
                            {
                                let mut immediate = lookup_result_signal.clone();
                                immediate.set(String::from("Looking up homeserver..."));
                            }
                            let logs_task = lookup_logs.clone();
                            let mut result_signal = lookup_result_signal.clone();
                            spawn(async move {
                                let pkdns = pubky_arc.pkdns();
                                let resolved = pkdns.get_homeserver_of(&target_pk).await;
                                match resolved {
                                    Some(host) => {
                                        result_signal.set(format!("Homeserver for {target_pk}: {host}"));
                                        logs_task.success(format!("Resolved homeserver for {target_pk}: {host}"));
                                    }
                                    None => {
                                        result_signal.set(format!("No homeserver record for {target_pk}"));
                                        logs_task.info(format!("No homeserver record for {target_pk}"));
                                    }
                                }
                            });
                        },
                        "Lookup public key",
                    }
                    button {
                        class: "action secondary",
                        title: "Check which homeserver the loaded key currently advertises",
                        onclick: move |_| {
                            let Some(kp) = self_lookup_keypair.read().as_ref().cloned() else {
                                self_lookup_logs.error("Load or generate a key first");
                                return;
                            };
                            let Some(pubky_arc) = self_lookup_pubky.ready_or_log(&self_lookup_logs) else {
                                return;
                            };
                            {
                                let mut immediate = self_lookup_result_signal.clone();
                                immediate.set(String::from("Checking homeserver for active key..."));
                            }
                            let logs_task = self_lookup_logs.clone();
                            let mut result_signal = self_lookup_result_signal.clone();
                            spawn(async move {
                                let signer = pubky_arc.signer(kp.clone());
                                let pkdns = signer.pkdns();
                                match pkdns.get_homeserver().await {
                                    Ok(Some(host)) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("Homeserver for {public}: {host}"));
                                        logs_task.success(format!("Active key advertises homeserver {host}"));
                                    }
                                    Ok(None) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("No homeserver record for {public}"));
                                        logs_task.info(format!(
                                            "No homeserver record found for active key {public}"
                                        ));
                                    }
                                    Err(err) => {
                                        result_signal.set(format!("Failed to resolve homeserver: {err}"));
                                        logs_task.error(format!("Failed to resolve homeserver: {err}"));
                                    }
                                }
                            });
                        },
                        "Lookup active key",
                    }
                }
                if !lookup_result_value.is_empty() {
                    div { class: "outputs", {lookup_result_value} }
                }
            }
            section { class: "card",
                h2 { "Publish homeserver" }
                p { class: "helper-text", "Publish or refresh your `_pubky` record. Leave the override blank to reuse the current host." }
                div { class: "form-grid",
                    label {
                        "Homeserver override (optional)"
                        input {
                            value: host_override_value,
                            oninput: move |evt| override_binding.set(evt.value()),
                            title: "Override the homeserver public key when publishing `_pubky` records",
                            placeholder: "Base32 homeserver public key",
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Publish `_pubky` if the existing record is missing or stale",
                        onclick: move |_| {
                            let Some(kp) = publish_if_stale_keypair.read().as_ref().cloned() else {
                                publish_if_stale_logs.error("Load or generate a key first");
                                return;
                            };
                            let Some(pubky_arc) = publish_if_stale_pubky.ready_or_log(&publish_if_stale_logs) else {
                                return;
                            };
                            let override_input = publish_if_stale_override.read().clone();
                            let override_value = override_input.trim();
                            let override_pk = if override_value.is_empty() {
                                None
                            } else {
                                match PublicKey::try_from(override_value) {
                                    Ok(pk) => Some(pk),
                                    Err(err) => {
                                        publish_if_stale_logs.error(format!("Invalid homeserver override: {err}"));
                                        return;
                                    }
                                }
                            };
                            {
                                let mut immediate = publish_if_stale_result_signal.clone();
                                immediate.set(String::from("Publishing homeserver (if stale)..."));
                            }
                            let logs_task = publish_if_stale_logs.clone();
                            let mut result_signal = publish_if_stale_result_signal.clone();
                            spawn(async move {
                                let signer = pubky_arc.signer(kp.clone());
                                let pkdns = signer.pkdns();
                                let override_for_task = override_pk.clone();
                                let publish_result = pkdns
                                    .publish_homeserver_if_stale(override_for_task.as_ref())
                                    .await;
                                if let Err(err) = publish_result {
                                    result_signal.set(format!("Failed to publish homeserver: {err}"));
                                    logs_task.error(format!("Failed to publish homeserver: {err}"));
                                    return;
                                }
                                match pkdns.get_homeserver().await {
                                    Ok(Some(host)) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("Homeserver for {public}: {host}"));
                                        if let Some(override_host) = override_for_task {
                                            logs_task.success(format!(
                                                "Published homeserver for {public} with override {override_host} -> {host}"
                                            ));
                                        } else {
                                            logs_task.success(format!(
                                                "Published homeserver for {public}: {host}"
                                            ));
                                        }
                                    }
                                    Ok(None) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("No homeserver record for {public}"));
                                        logs_task.info(format!(
                                            "No homeserver record published for {public} (missing host)"
                                        ));
                                    }
                                    Err(err) => {
                                        result_signal.set(format!("Failed to read homeserver: {err}"));
                                        logs_task.error(format!("Failed to read homeserver: {err}"));
                                    }
                                }
                            });
                        },
                        "Publish if stale",
                    }
                    button {
                        class: "action secondary",
                        title: "Force a `_pubky` publish even if the record is fresh",
                        onclick: move |_| {
                            let Some(kp) = publish_force_keypair.read().as_ref().cloned() else {
                                publish_force_logs.error("Load or generate a key first");
                                return;
                            };
                            let Some(pubky_arc) = publish_force_pubky.ready_or_log(&publish_force_logs) else {
                                return;
                            };
                            let override_input = publish_force_override.read().clone();
                            let override_value = override_input.trim();
                            let override_pk = if override_value.is_empty() {
                                None
                            } else {
                                match PublicKey::try_from(override_value) {
                                    Ok(pk) => Some(pk),
                                    Err(err) => {
                                        publish_force_logs.error(format!("Invalid homeserver override: {err}"));
                                        return;
                                    }
                                }
                            };
                            {
                                let mut immediate = publish_force_result_signal.clone();
                                immediate.set(String::from("Publishing homeserver (force)..."));
                            }
                            let logs_task = publish_force_logs.clone();
                            let mut result_signal = publish_force_result_signal.clone();
                            spawn(async move {
                                let signer = pubky_arc.signer(kp.clone());
                                let pkdns = signer.pkdns();
                                let override_for_task = override_pk.clone();
                                let publish_result = pkdns
                                    .publish_homeserver_force(override_for_task.as_ref())
                                    .await;
                                if let Err(err) = publish_result {
                                    result_signal.set(format!("Failed to publish homeserver: {err}"));
                                    logs_task.error(format!("Failed to publish homeserver: {err}"));
                                    return;
                                }
                                match pkdns.get_homeserver().await {
                                    Ok(Some(host)) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("Homeserver for {public}: {host}"));
                                        if let Some(override_host) = override_for_task {
                                            logs_task.success(format!(
                                                "Force-published homeserver for {public} with override {override_host} -> {host}"
                                            ));
                                        } else {
                                            logs_task.success(format!(
                                                "Force-published homeserver for {public}: {host}"
                                            ));
                                        }
                                    }
                                    Ok(None) => {
                                        let public = kp.public_key();
                                        result_signal.set(format!("No homeserver record for {public}"));
                                        logs_task.info(format!(
                                            "No homeserver record published for {public} (missing host)"
                                        ));
                                    }
                                    Err(err) => {
                                        result_signal.set(format!("Failed to read homeserver: {err}"));
                                        logs_task.error(format!("Failed to read homeserver: {err}"));
                                    }
                                }
                            });
                        },
                        "Force publish",
                    }
                }
            }
        }
    }
}
