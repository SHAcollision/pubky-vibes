use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Url;
use serde::Deserialize;
use tracing::warn;

#[derive(Clone, Debug)]
pub(crate) struct AdminClient {
    http: reqwest::Client,
}

impl AdminClient {
    pub(crate) fn new() -> Self {
        let builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .pool_max_idle_per_host(4);

        let http = match builder.build() {
            Ok(client) => client,
            Err(err) => {
                warn!(
                    ?err,
                    "Failed to build admin HTTP client with tuned settings; using defaults"
                );
                reqwest::Client::new()
            }
        };

        Self { http }
    }

    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AdminInfo {
    pub(crate) num_users: u64,
    pub(crate) num_disabled_users: u64,
    pub(crate) total_disk_used_mb: f64,
    pub(crate) num_signup_codes: u64,
    pub(crate) num_unused_signup_codes: u64,
}

pub(crate) async fn fetch_info(
    client: &AdminClient,
    base_url: &str,
    password: &str,
) -> Result<AdminInfo> {
    let url = endpoint(base_url, "/info")?;
    let response = client
        .http()
        .get(url)
        .header("X-Admin-Password", password.trim())
        .send()
        .await
        .context("Failed to reach the admin info endpoint")?
        .error_for_status()
        .context("Admin server rejected the info request")?;

    response
        .json::<AdminInfo>()
        .await
        .context("Failed to parse info response")
}

pub(crate) async fn generate_signup_token(
    client: &AdminClient,
    base_url: &str,
    password: &str,
) -> Result<String> {
    let url = endpoint(base_url, "/generate_signup_token")?;
    let response = client
        .http()
        .get(url)
        .header("X-Admin-Password", password.trim())
        .send()
        .await
        .context("Failed to reach the generate_signup_token endpoint")?
        .error_for_status()
        .context("Admin server rejected the signup token request")?;

    response
        .text()
        .await
        .context("Failed to read signup token response body")
}

pub(crate) async fn delete_entry(
    client: &AdminClient,
    base_url: &str,
    password: &str,
    entry_path: &str,
) -> Result<()> {
    let url = endpoint(base_url, &format!("/webdav/{}", entry_path))?;
    client
        .http()
        .delete(url)
        .header("X-Admin-Password", password.trim())
        .send()
        .await
        .context("Failed to reach the delete entry endpoint")?
        .error_for_status()
        .context("Admin server rejected the delete entry request")?;

    Ok(())
}

pub(crate) async fn toggle_user_disabled(
    client: &AdminClient,
    base_url: &str,
    password: &str,
    pubkey: &str,
    disable: bool,
) -> Result<()> {
    let action = if disable { "disable" } else { "enable" };
    let url = endpoint(base_url, &format!("/users/{pubkey}/{action}"))?;
    client
        .http()
        .post(url)
        .header("X-Admin-Password", password.trim())
        .send()
        .await
        .context("Failed to reach the user toggle endpoint")?
        .error_for_status()
        .context("Admin server rejected the user toggle request")?;

    Ok(())
}

fn endpoint(base_url: &str, path: &str) -> Result<Url> {
    let url = Url::parse(base_url).context("Invalid admin base URL")?;
    url.join(path).context("Invalid admin endpoint path")
}
