use std::sync::Arc;

use anyhow::Result;
use pubky::Pubky;

use crate::models::{MatchStoragePaths, TurnSnapshot};
use crate::services::storage::{fetch_latest_pointer, fetch_turn};

pub async fn discover_new_turn(
    facade: Arc<Pubky>,
    opponent_public_key: &str,
    paths: &MatchStoragePaths,
    known_turn: Option<u32>,
) -> Result<Option<TurnSnapshot>> {
    let latest_url = format!("pubky{opponent_public_key}{}", paths.latest_path());
    let Some(pointer) = fetch_latest_pointer(facade.clone(), &latest_url).await? else {
        return Ok(None);
    };
    if Some(pointer.turn) == known_turn || pointer.turn <= known_turn.unwrap_or(0) {
        return Ok(None);
    }
    let namespace = paths.namespace.trim_start_matches('/');
    let snapshot = fetch_turn(facade, opponent_public_key, namespace, &pointer.file).await?;
    Ok(Some(snapshot))
}
