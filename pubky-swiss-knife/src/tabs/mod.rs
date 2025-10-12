pub mod auth;
pub mod http;
pub mod keys;
pub mod sessions;
pub mod state;
pub mod storage;
pub mod tokens;

pub use auth::render_auth_tab;
pub use http::render_http_tab;
pub use keys::render_keys_tab;
pub use sessions::render_sessions_tab;
pub use state::{
    AuthTabState, HttpTabState, KeysTabState, SessionsTabState, StorageTabState, TokensTabState,
};
pub use storage::render_storage_tab;
pub use tokens::render_tokens_tab;

pub fn format_session_info(info: &impl std::fmt::Debug) -> String {
    format!("{info:#?}")
}
