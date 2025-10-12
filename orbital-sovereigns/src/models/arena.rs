use indexmap::IndexMap;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::ship::ShipBlueprint;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn scale(self, factor: f32) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipState {
    pub id: Uuid,
    pub blueprint_signature: String,
    pub position: Vector3,
    pub velocity: Vector3,
    pub hull: i32,
    pub shield: i32,
}

impl ShipState {
    pub fn from_blueprint(blueprint: &ShipBlueprint) -> Self {
        Self {
            id: Uuid::new_v4(),
            blueprint_signature: blueprint.signature(),
            position: Vector3::ZERO,
            velocity: Vector3::ZERO,
            hull: blueprint.hull_integrity() as i32,
            shield: (blueprint.total_mass() / 3) as i32,
        }
    }

    pub fn apply_damage(&mut self, damage: i32) {
        let mut remaining = damage;
        if self.shield > 0 {
            let absorbed = self.shield.min(remaining);
            self.shield -= absorbed;
            remaining -= absorbed;
        }
        if remaining > 0 {
            self.hull -= remaining;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArenaState {
    pub seed: u64,
    pub round: u32,
    pub ships: IndexMap<String, ShipState>,
}

impl ArenaState {
    pub fn new(seed: u64, ships: IndexMap<String, ShipState>) -> Self {
        Self {
            seed,
            round: 0,
            ships,
        }
    }

    pub fn alive_ships(&self) -> usize {
        self.ships.values().filter(|ship| ship.hull > 0).count()
    }

    pub fn is_match_over(&self) -> bool {
        self.alive_ships() <= 1
    }

    pub fn roll_rng(&self) -> SmallRng {
        SmallRng::seed_from_u64(self.seed + self.round as u64)
    }

    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.seed.to_be_bytes());
        hasher.update(self.round.to_be_bytes());
        for (pilot, ship) in &self.ships {
            hasher.update(pilot.as_bytes());
            hasher.update(ship.id.as_bytes());
            hasher.update(ship.position.x.to_be_bytes());
            hasher.update(ship.position.y.to_be_bytes());
            hasher.update(ship.position.z.to_be_bytes());
            hasher.update(ship.velocity.x.to_be_bytes());
            hasher.update(ship.velocity.y.to_be_bytes());
            hasher.update(ship.velocity.z.to_be_bytes());
            hasher.update(ship.hull.to_be_bytes());
            hasher.update(ship.shield.to_be_bytes());
        }
        hex::encode(hasher.finalize())
    }

    pub fn advance_round(&mut self) {
        self.round += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_damage_applies_to_shields_first() {
        let blueprint = ShipBlueprint {
            name: "Test".into(),
            modules: vec![],
            crew: 3,
            mass_override: Some(120),
        };
        let mut ship = ShipState::from_blueprint(&blueprint);
        ship.shield = 50;
        ship.hull = 100;
        ship.apply_damage(60);
        assert_eq!(ship.shield, 0);
        assert_eq!(ship.hull, 90);
    }
}
