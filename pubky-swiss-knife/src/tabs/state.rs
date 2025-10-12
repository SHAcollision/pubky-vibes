use dioxus::prelude::Signal;
use pubky::{Keypair, PubkyAuthFlow, PubkySession};

#[derive(Clone)]
pub struct KeysTabState {
    pub keypair: Signal<Option<Keypair>>,
    pub secret_input: Signal<String>,
    pub recovery_path: Signal<String>,
    pub recovery_passphrase: Signal<String>,
}

#[derive(Clone)]
pub struct TokensTabState {
    pub keypair: Signal<Option<Keypair>>,
    pub capabilities: Signal<String>,
    pub output: Signal<String>,
}

#[derive(Clone)]
pub struct SessionsTabState {
    pub keypair: Signal<Option<Keypair>>,
    pub session: Signal<Option<PubkySession>>,
    pub details: Signal<String>,
    pub homeserver: Signal<String>,
    pub signup_code: Signal<String>,
}

#[derive(Clone)]
pub struct AuthTabState {
    pub keypair: Signal<Option<Keypair>>,
    pub session: Signal<Option<PubkySession>>,
    pub details: Signal<String>,
    pub capabilities: Signal<String>,
    pub relay: Signal<String>,
    pub url_output: Signal<String>,
    pub qr_data: Signal<Option<String>>,
    pub status: Signal<String>,
    pub flow: Signal<Option<PubkyAuthFlow>>,
    pub request_body: Signal<String>,
}

#[derive(Clone)]
pub struct StorageTabState {
    pub session: Signal<Option<PubkySession>>,
    pub path: Signal<String>,
    pub body: Signal<String>,
    pub response: Signal<String>,
    pub public_resource: Signal<String>,
    pub public_response: Signal<String>,
}

#[derive(Clone)]
pub struct HttpTabState {
    pub method: Signal<String>,
    pub url: Signal<String>,
    pub headers: Signal<String>,
    pub body: Signal<String>,
    pub response: Signal<String>,
}
