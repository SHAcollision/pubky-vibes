use anyhow::{Context, Result};
use rand::distributions::{Distribution, Uniform};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::arena::{ArenaState, Vector3};
use super::match_state::MatchMeta;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BattleAction {
    Maneuver { thrust: Vector3 },
    Fire { target: String, power: u16 },
    Brace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BattleEvent {
    Movement { actor: String, delta: Vector3 },
    Damage { target: String, amount: i32 },
    Miss { target: String },
    Brace { actor: String, restored: i32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnSnapshot {
    pub turn: u32,
    pub actor: String,
    pub action: BattleAction,
    pub events: Vec<BattleEvent>,
    pub state_hash: String,
    pub applied_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchState {
    pub meta: MatchMeta,
    pub arena: ArenaState,
    pub turns: Vec<TurnSnapshot>,
}

impl MatchState {
    pub fn new(meta: MatchMeta, arena: ArenaState) -> Self {
        Self {
            meta,
            arena,
            turns: Vec::new(),
        }
    }

    pub fn latest_turn(&self) -> Option<&TurnSnapshot> {
        self.turns.last()
    }

    pub fn apply_action(&mut self, actor: &str, action: BattleAction) -> Result<TurnSnapshot> {
        self.meta.ensure_guest()?;
        let mut events = Vec::new();
        let mut rng = self.arena.roll_rng();
        let (actor_shield, actor_hull) = {
            let ship = self
                .arena
                .ships
                .get(actor)
                .with_context(|| format!("No ship registered for actor {actor}"))?;
            if ship.hull <= 0 {
                anyhow::bail!("{actor} is already destroyed");
            }
            (ship.shield, ship.hull)
        };

        match &action {
            BattleAction::Maneuver { thrust } => {
                let ship = self
                    .arena
                    .ships
                    .get_mut(actor)
                    .with_context(|| format!("No ship registered for actor {actor}"))?;
                ship.velocity = ship.velocity.add(thrust.scale(0.6));
                ship.position = ship.position.add(ship.velocity);
                events.push(BattleEvent::Movement {
                    actor: actor.to_string(),
                    delta: *thrust,
                });
            }
            BattleAction::Fire { target, power } => {
                let Some(target_ship) = self.arena.ships.get_mut(target) else {
                    anyhow::bail!("Target {target} not found");
                };
                if target_ship.hull <= 0 {
                    anyhow::bail!("Target {target} is already destroyed");
                }
                let hit_chance = (actor_shield.max(0) + actor_hull.max(0)) as f32
                    / ((actor_shield.max(0)
                        + actor_hull.max(0)
                        + target_ship.shield.max(0)
                        + target_ship.hull.max(0)) as f32)
                    * 0.75
                    + 0.15;
                let roll = Uniform::new(0.0f32, 1.0f32).sample(&mut rng);
                if roll <= hit_chance {
                    let damage_roll = Uniform::new_inclusive(4i32, 17i32).sample(&mut rng);
                    let damage = (*power as i32) + damage_roll;
                    target_ship.apply_damage(damage);
                    events.push(BattleEvent::Damage {
                        target: target.clone(),
                        amount: damage,
                    });
                } else {
                    events.push(BattleEvent::Miss {
                        target: target.clone(),
                    });
                }
            }
            BattleAction::Brace => {
                let ship = self
                    .arena
                    .ships
                    .get_mut(actor)
                    .with_context(|| format!("No ship registered for actor {actor}"))?;
                let restored = Uniform::new_inclusive(6, 17).sample(&mut rng);
                ship.shield += restored;
                events.push(BattleEvent::Brace {
                    actor: actor.to_string(),
                    restored,
                });
            }
        }

        self.arena.advance_round();
        let snapshot = TurnSnapshot {
            turn: self.arena.round,
            actor: actor.to_string(),
            action,
            events,
            state_hash: self.arena.digest(),
            applied_at: OffsetDateTime::now_utc(),
        };
        self.turns.push(snapshot.clone());
        Ok(snapshot)
    }

    pub fn rebuild_arena(&mut self) -> Result<()> {
        let mut arena = self.meta.arena_state()?;
        for turn in &self.turns {
            self.apply_action_inner(&mut arena, turn)?;
            arena.advance_round();
        }
        self.arena = arena;
        Ok(())
    }

    fn apply_action_inner(&self, arena: &mut ArenaState, turn: &TurnSnapshot) -> Result<()> {
        let ship = arena
            .ships
            .get_mut(&turn.actor)
            .with_context(|| format!("Unknown ship {} during replay", turn.actor))?;
        match &turn.action {
            BattleAction::Maneuver { thrust } => {
                ship.velocity = ship.velocity.add(thrust.scale(0.6));
                ship.position = ship.position.add(ship.velocity);
            }
            BattleAction::Fire { target, .. } => {
                if let Some(target_ship) = arena.ships.get_mut(target) {
                    for event in &turn.events {
                        if let BattleEvent::Damage { amount, .. } = event {
                            target_ship.apply_damage(*amount);
                        }
                    }
                }
            }
            BattleAction::Brace => {
                for event in &turn.events {
                    if let BattleEvent::Brace { restored, .. } = event {
                        ship.shield += *restored;
                    }
                }
            }
        }
        Ok(())
    }
}
