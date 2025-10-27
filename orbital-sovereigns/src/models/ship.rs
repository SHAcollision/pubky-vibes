use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::identity::VaultEnvelope;

#[repr(u8)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleKind {
    Hull,
    Reactor,
    Thruster,
    Cannon,
    Shield,
    Hangar,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipModule {
    pub id: Uuid,
    pub kind: ModuleKind,
    pub position: [i8; 3],
    pub facing: [i8; 3],
    pub power: u16,
    pub mass: u16,
}

impl ShipModule {
    pub fn new(
        kind: ModuleKind,
        position: [i8; 3],
        facing: [i8; 3],
        power: u16,
        mass: u16,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            position,
            facing,
            power,
            mass,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipBlueprint {
    pub name: String,
    pub modules: Vec<ShipModule>,
    pub crew: u16,
    pub mass_override: Option<u32>,
}

impl ShipBlueprint {
    pub fn total_mass(&self) -> u32 {
        if let Some(override_mass) = self.mass_override {
            return override_mass;
        }
        self.modules.iter().map(|module| module.mass as u32).sum()
    }

    pub fn hull_integrity(&self) -> u32 {
        self.modules
            .iter()
            .filter(|module| matches!(module.kind, ModuleKind::Hull | ModuleKind::Shield))
            .map(|module| (module.power.max(1) as u32) * 3)
            .sum()
    }

    pub fn signature(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        for module in &self.modules {
            hasher.update(module.id.as_bytes());
            for coord in module.position {
                hasher.update(&coord.to_be_bytes());
            }
            for coord in module.facing {
                hasher.update(&coord.to_be_bytes());
            }
            hasher.update(module.power.to_be_bytes());
            hasher.update(module.mass.to_be_bytes());
            hasher.update([module.kind.clone() as u8]);
        }
        hasher.update(self.crew.to_be_bytes());
        hasher.update(self.total_mass().to_be_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
    }

    pub fn encrypt(&self, key: &[u8; 32]) -> VaultEnvelope {
        let serialized = serde_json::to_vec(self).expect("Blueprints serialize");
        VaultEnvelope::seal(key, &serialized)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlueprintVault {
    pub blueprint_name: String,
    pub envelope: VaultEnvelope,
}

impl BlueprintVault {
    pub fn new(blueprint: &ShipBlueprint, key_material: &[u8; 32]) -> Self {
        Self {
            blueprint_name: blueprint.name.clone(),
            envelope: blueprint.encrypt(key_material),
        }
    }
}
