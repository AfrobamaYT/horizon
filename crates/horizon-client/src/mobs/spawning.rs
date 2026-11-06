//! Natural spawning rules: `SpawnPlacements` placement types and each
//! species' spawn predicate (`checkAnimalSpawnRules`, `checkMonsterSpawnRules`,
//! water animals, bats...), and `finalizeSpawn` (variants, colours, babies,
//! equipment).
use super::{Mob, MobWorld, Species, farm::Climate};

/// Biome facts finalizeSpawn and the spawn rules read.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiomeFacts {
    pub climate: Climate,
    /// Biome temperature at the position (strays in cold biomes, snow foxes).
    pub temperature: f32,
    pub snowy: bool,
    /// `#minecraft:spawns_snow_foxes` (`Fox.Variant.byBiome`: snow, else red).
    pub snow_fox: bool,
    pub swamp: bool,
    pub ocean: bool,
    pub mushroom: bool,
    pub desert: bool,
    pub jungle: bool,
    /// Frog variant from `spawns_warm/cold_variant_frogs` (0 temperate, 1 warm, 2 cold).
    pub frog: u8,
    /// Wolf variant index (`tame::WOLF_VARIANTS`).
    pub wolf: u8,
    /// `MoonBrightnessCheck(>= 0.9)`: the full moon.
    pub full_moon: bool,
    /// `spawns_coral_variant_zombie_nautilus` (the warm ocean).
    pub coral_nautilus: bool,
}
/// `SpawnPlacements.isSpawnPositionOk` for the species.
pub fn placement_ok(species: Species, w: &MobWorld, x: i32, y: i32, z: i32) -> bool {
    super::spawn_rules::is_spawn_position_ok(species, w, x, y, z)
}
/// Mob.finalizeSpawn for natural, chunk-generation and summoned mobs.
/// `DifficultyInstance`: the local difficulty at a position.
#[derive(Clone, Copy, Debug)]
pub struct LocalDifficulty {
    /// The world difficulty id, 0 peaceful to 3 hard.
    pub base: u8,
    pub effective: f32,
    /// The world difficulty is normal.
    pub normal: bool,
    /// The world difficulty is hard.
    pub hard: bool,
    /// `Difficulty.getId`: 0 peaceful to 3 hard.
    pub id: u8,
}
impl From<LocalDifficulty> for mod_api::NativeDifficulty {
    fn from(d: LocalDifficulty) -> Self {
        Self::new(d.base, d.effective)
    }
}
impl From<mod_api::NativeDifficulty> for LocalDifficulty {
    /// A mod's difficulty; a base past hard is hard.
    fn from(d: mod_api::NativeDifficulty) -> Self {
        let base = d.base.min(3);
        Self {
            base,
            effective: d.effective,
            normal: base == 2,
            hard: base == 3,
            id: base,
        }
    }
}
/// `DimensionType.MOON_BRIGHTNESS_PER_PHASE`.
pub const MOON_BRIGHTNESS: [f32; 8] = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];
impl LocalDifficulty {
    /// `calculateDifficulty` from the difficulty id (0 peaceful to 3 hard),
    /// the overworld clock, the chunk's inhabited time and the moon.
    pub fn new(base: u8, total: i64, inhabited: i64, moon: f32) -> Self {
        let hard = base == 3;
        let normal = base == 2;
        if base == 0 {
            return Self {
                base,
                effective: 0.0,
                normal,
                hard,
                id: base,
            };
        }
        let global = ((total as f32 - 72000.0) / 1_440_000.0).clamp(0.0, 1.0) * 0.25;
        let mut local =
            (inhabited as f32 / 3_600_000.0).clamp(0.0, 1.0) * if hard { 1.0 } else { 0.75 };
        local += (moon * 0.25).clamp(0.0, global);
        if base == 1 {
            local *= 0.5;
        }
        Self {
            base,
            effective: f32::from(base) * (0.75 + global + local),
            normal,
            hard,
            id: base,
        }
    }
    /// `getSpecialMultiplier`.
    pub fn special(&self) -> f32 {
        if self.effective < 2.0 {
            0.0
        } else if self.effective > 4.0 {
            1.0
        } else {
            (self.effective - 2.0) / 2.0
        }
    }
}
/// `Mob.populateDefaultEquipmentSlots`: sometimes a set of armour, from the
/// head down, of one random material.
/// A baby from `Animal.spawnChildFromBreeding`: `getBreedOffspring` creates
/// it without `finalizeSpawn`, so it never ran `Mob.finalizeSpawn`. Undoes
/// that part of finalizeSpawn: no `random_spawn_bonus`, right-handed.
pub fn bred_offspring(m: &mut Mob) {
    use super::finalize::{Attr, remove_modifier};
    remove_modifier(m, Attr::FollowRange, "minecraft:random_spawn_bonus");
    m.extra.left_handed = false;
}
/// VillagerType.byBiome
pub fn villager_type(b: &BiomeFacts) -> u8 {
    if b.desert {
        1
    } else if b.jungle {
        2
    } else if b.snowy {
        4
    } else if b.swamp {
        5
    } else if b.climate == Climate::Warm {
        3
    } else if b.climate == Climate::Cold && b.temperature < 0.4 {
        6
    } else {
        0
    }
}
