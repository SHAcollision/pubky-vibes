use anyhow::Result;
use pubky::PubkySession;
use reqwest::Response;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::models::{MatchMeta, MatchStoragePaths, TurnSnapshot};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatestPointer {
    pub turn: u32,
    pub hash: String,
    pub file: String,
}

pub async fn store_match_meta(
    session: &PubkySession,
    paths: &MatchStoragePaths,
    meta: &MatchMeta,
) -> Result<()> {
    let body = serde_json::to_string_pretty(meta)?;
    session
        .storage()
        .put(paths.meta_path(), body)
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn store_turn(
    session: &PubkySession,
    paths: &MatchStoragePaths,
    snapshot: &TurnSnapshot,
) -> Result<LatestPointer> {
    let turn_path = paths.move_path(snapshot.turn, &snapshot.actor);
    let body = serde_json::to_string_pretty(snapshot)?;
    session
        .storage()
        .put(turn_path.clone(), body)
        .await?
        .error_for_status()?;
    let pointer = LatestPointer {
        turn: snapshot.turn,
        hash: snapshot.state_hash.clone(),
        file: turn_path
            .trim_start_matches(&paths.namespace)
            .trim_start_matches('/')
            .to_string(),
    };
    let pointer_body = serde_json::to_string_pretty(&pointer)?;
    session
        .storage()
        .put(paths.latest_path(), pointer_body)
        .await?
        .error_for_status()?;
    Ok(pointer)
}

pub async fn fetch_latest_pointer(
    facade: Arc<pubky::Pubky>,
    url: &str,
) -> Result<Option<LatestPointer>> {
    let response = facade.public_storage().get(url.to_string()).await?;
    if response.status().is_client_error() {
        return Ok(None);
    }
    let pointer = read_json::<LatestPointer>(response).await?;
    Ok(Some(pointer))
}

pub async fn fetch_turn(
    facade: Arc<pubky::Pubky>,
    owner: &str,
    namespace: &str,
    file: &str,
) -> Result<TurnSnapshot> {
    let path = format!("pubky{owner}/{namespace}/{file}");
    let response = facade.public_storage().get(path).await?;
    read_json(response).await
}

pub async fn read_json<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Request failed with status {status}");
    }
    Ok(response.json::<T>().await?)
}

pub async fn read_string(response: Response) -> Result<String> {
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Request failed with status {status}");
    }
    Ok(response.text().await?)
}
