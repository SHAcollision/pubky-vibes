pub mod arena;
pub mod battle;
pub mod identity;
pub mod match_state;
pub mod ship;

pub use arena::{ArenaState, ShipState, Vector3};
pub use battle::{BattleAction, BattleEvent, MatchState, TurnSnapshot};
pub use identity::{CommanderIdentity, CommanderProfile, VaultEnvelope};
pub use match_state::{MatchDescriptor, MatchMeta, MatchRules, MatchStoragePaths};
pub use ship::{BlueprintVault, ModuleKind, ShipBlueprint, ShipModule};
