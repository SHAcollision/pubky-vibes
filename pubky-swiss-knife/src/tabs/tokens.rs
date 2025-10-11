use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use pubky::{AuthToken, Capabilities, Keypair};

use crate::utils::logging::{LogEntry, LogLevel, push_log};

pub fn render_tokens_tab(
    keypair: Signal<Option<Keypair>>,
    token_caps_input: Signal<String>,
    token_output: Signal<String>,
    logs: Signal<Vec<LogEntry>>,
) -> Element {
    let caps_value = { token_caps_input.read().clone() };
    let token_value = { token_output.read().clone() };

    let mut token_caps_binding = token_caps_input;

    let sign_keypair = keypair;
    let sign_caps = token_caps_input;
    let mut sign_token = token_output;
    let sign_logs = logs;

    rsx! {
        div { class: "tab-body single-column",
            section { class: "card",
                h2 { "Sign capability tokens" }
                p { class: "helper-text", "Compose a capability string (e.g. '/:rw,/pub/app/:r') and sign using the active key." }
                div { class: "form-grid",
                    label {
                        "Capabilities"
                        input {
                            value: caps_value,
                            oninput: move |evt| token_caps_binding.set(evt.value()),
                            title: "Enter the capabilities you want to grant, separated by commas",
                            placeholder: "Comma-separated scopes"
                        }
                    }
                }
                div { class: "small-buttons",
                    button {
                        class: "action",
                        title: "Sign the listed scopes with the currently loaded key",
                        onclick: move |_| {
                            let caps = sign_caps.read().clone();
                            if let Some(kp) = sign_keypair.read().as_ref() {
                                match Capabilities::try_from(caps.as_str()) {
                                    Ok(capabilities) => {
                                        let token = AuthToken::sign(kp, capabilities.clone());
                                        sign_token.set(STANDARD.encode(token.serialize()));
                                        push_log(sign_logs, LogLevel::Success, format!(
                                            "Signed token for {} with caps {capabilities}",
                                            kp.public_key()
                                        ));
                                    }
                                    Err(err) => push_log(sign_logs, LogLevel::Error, format!("Invalid capabilities: {err}")),
                                }
                            } else {
                                push_log(sign_logs, LogLevel::Error, "Load a key before signing");
                            }
                        },
                        "Sign token"
                    }
                }
                if !token_value.is_empty() {
                    div { class: "outputs", {token_value} }
                }
            }
        }
    }
}
