use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use pubky::{AuthToken, Capabilities};

use crate::tabs::TokensTabState;
use crate::utils::logging::ActivityLog;

pub fn render_tokens_tab(state: TokensTabState, logs: ActivityLog) -> Element {
    let TokensTabState {
        keypair,
        capabilities,
        output,
    } = state;

    let caps_value = { capabilities.read().clone() };
    let token_value = { output.read().clone() };

    let mut token_caps_binding = capabilities;

    let sign_keypair = keypair;
    let sign_caps = capabilities;
    let mut sign_token = output;
    let sign_logs = logs.clone();

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
                                        sign_logs.success(format!(
                                            "Signed token for {} with caps {capabilities}",
                                            kp.public_key()
                                        ));
                                    }
                                    Err(err) => sign_logs.error(format!("Invalid capabilities: {err}")),
                                }
                            } else {
                                sign_logs.error("Load a key before signing");
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
