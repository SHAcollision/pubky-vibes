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
pub struct PkdnsTabState {
    pub keypair: Signal<Option<Keypair>>,
    pub lookup_input: Signal<String>,
    pub lookup_result: Signal<String>,
    pub host_override: Signal<String>,
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

#[derive(Clone)]
pub struct SocialTabState {
    pub session: Signal<Option<PubkySession>>,
    pub profile_name: Signal<String>,
    pub profile_bio: Signal<String>,
    pub profile_image: Signal<String>,
    pub profile_status: Signal<String>,
    pub profile_links: Signal<String>,
    pub profile_error: Signal<String>,
    pub profile_response: Signal<String>,
    pub post_content: Signal<String>,
    pub post_kind: Signal<String>,
    pub post_parent: Signal<String>,
    pub post_embed_kind: Signal<String>,
    pub post_embed_uri: Signal<String>,
    pub post_attachments: Signal<String>,
    pub post_response: Signal<String>,
    pub tag_uri: Signal<String>,
    pub tag_label: Signal<String>,
    pub tag_response: Signal<String>,
}
