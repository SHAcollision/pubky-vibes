use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::arena::{ArenaState, ShipState};
use super::identity::CommanderProfile;
use super::ship::ShipBlueprint;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchRules {
    pub max_rounds: u32,
    pub arena_radius: f32,
    pub simultaneous_fire: bool,
}

impl Default for MatchRules {
    fn default() -> Self {
        Self {
            max_rounds: 60,
            arena_radius: 520.0,
            simultaneous_fire: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchMeta {
    pub match_id: Uuid,
    pub created_at: OffsetDateTime,
    pub host: CommanderProfile,
    pub guest: Option<CommanderProfile>,
    pub rules: MatchRules,
    pub seed: u64,
    pub host_blueprint: ShipBlueprint,
    pub guest_blueprint: Option<ShipBlueprint>,
}

impl MatchMeta {
    pub fn new(
        host: CommanderProfile,
        host_blueprint: ShipBlueprint,
        rules: MatchRules,
        seed: u64,
    ) -> Self {
        Self {
            match_id: Uuid::new_v4(),
            created_at: OffsetDateTime::now_utc(),
            host,
            guest: None,
            rules,
            seed,
            host_blueprint,
            guest_blueprint: None,
        }
    }

    pub fn with_guest(mut self, guest: CommanderProfile, guest_blueprint: ShipBlueprint) -> Self {
        self.guest = Some(guest);
        self.guest_blueprint = Some(guest_blueprint);
        self
    }

    pub fn arena_state(&self) -> Result<ArenaState> {
        let mut ships = IndexMap::new();
        ships.insert(
            self.host.label.clone(),
            ShipState::from_blueprint(&self.host_blueprint),
        );
        if let (Some(guest), Some(bp)) = (&self.guest, &self.guest_blueprint) {
            ships.insert(guest.label.clone(), ShipState::from_blueprint(bp));
        }
        Ok(ArenaState::new(self.seed, ships))
    }

    pub fn ensure_guest(&self) -> Result<()> {
        if self.guest.is_some() && self.guest_blueprint.is_some() {
            Ok(())
        } else {
            anyhow::bail!("Match is not ready until a guest joins with a blueprint")
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchDescriptor {
    pub meta: MatchMeta,
    pub latest_turn: Option<u32>,
}

impl MatchDescriptor {
    pub fn new(meta: MatchMeta) -> Self {
        Self {
            meta,
            latest_turn: None,
        }
    }

    pub fn bump_turn(&mut self, turn: u32) {
        self.latest_turn = Some(turn);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchStoragePaths {
    pub match_id: Uuid,
    pub namespace: String,
}

impl MatchStoragePaths {
    pub fn new(match_id: Uuid) -> Self {
        Self {
            match_id,
            namespace: format!("/pub/orbital-sovereigns/matches/{match_id}"),
        }
    }

    pub fn meta_path(&self) -> String {
        format!("{}/meta.json", self.namespace)
    }

    pub fn latest_path(&self) -> String {
        format!("{}/latest.json", self.namespace)
    }

    pub fn move_path(&self, turn: u32, actor: &str) -> String {
        let normalized = actor
            .to_ascii_lowercase()
            .replace(' ', "-")
            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "");
        format!("{}/moves/{:04}-{normalized}.json", self.namespace, turn)
    }

    pub fn chat_path(&self, epoch_ms: i128) -> String {
        format!("{}/chat/{epoch_ms}.txt", self.namespace)
    }
}
