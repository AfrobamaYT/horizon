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
/// The armour and toughness each vanilla armour piece adds.
pub fn armor_stats(item: &str) -> (f32, f32) {
    let Some(name) = item.strip_prefix("minecraft:") else {
        return (0.0, 0.0);
    };
    let slot = ["helmet", "chestplate", "leggings", "boots"]
        .iter()
        .position(|p| name.ends_with(p));
    let Some(slot) = slot else { return (0.0, 0.0) };
    let (points, toughness): ([f32; 4], f32) = match name.split('_').next() {
        Some("leather") => ([1.0, 3.0, 2.0, 1.0], 0.0),
        Some("copper") => ([2.0, 4.0, 3.0, 1.0], 0.0),
        Some("golden") => ([2.0, 5.0, 3.0, 1.0], 0.0),
        Some("chainmail") => ([2.0, 5.0, 4.0, 1.0], 0.0),
        Some("iron") => ([2.0, 6.0, 5.0, 2.0], 0.0),
        Some("turtle") => ([2.0, 0.0, 0.0, 0.0], 0.0),
        Some("diamond") => ([3.0, 8.0, 6.0, 3.0], 2.0),
        Some("netherite") => ([3.0, 8.0, 6.0, 3.0], 3.0),
        _ => return (0.0, 0.0),
    };
    (points[slot], toughness)
}
/// The armour and toughness a horse armor item adds in the body slot
/// (`ArmorMaterial.createAttributes(ArmorType.BODY)`).
pub fn horse_armor_stats(item: &str) -> (f32, f32) {
    match item.strip_prefix("minecraft:") {
        Some("leather_horse_armor") => (3.0, 0.0),
        Some("copper_horse_armor") => (4.0, 0.0),
        Some("iron_horse_armor") => (5.0, 0.0),
        Some("golden_horse_armor") => (7.0, 0.0),
        Some("diamond_horse_armor") => (11.0, 2.0),
        Some("netherite_horse_armor") => (19.0, 3.0),
        _ => (0.0, 0.0),
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::finalize::{Attr, has_modifier, value};

    fn types() -> Option<crate::mobs::MobTypes> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/horizon-client")
            .join(horizon_core::VANILLA_VERSION);
        if !root.join("items.json").exists() {
            return None;
        }
        Some(crate::mobs::MobTypes::load(&root).unwrap())
    }
    fn spawn(types: &crate::mobs::MobTypes, kind: &str, id: u32) -> Mob {
        let kind = std::sync::Arc::new(types.types.get(kind).unwrap().clone());
        let mut m = Mob::new(
            id,
            kind,
            horizon_core::geometry::Vec3::new(0.5, 64.0, 0.5),
            0.0,
        );
        let difficulty = LocalDifficulty::new(3, 0, 0, 1.0);
        finalize(&mut m, &BiomeFacts::default(), &difficulty);
        m
    }
    /// #2783: `/summon`, eggs, spawners and natural spawns run
    /// `WitherSkeleton.finalizeSpawn`: attack damage base 4.0, not the
    /// default 2.0.
    #[test]
    fn runtime_wither_skeletons_hit_for_four() {
        let Some(types) = types() else { return };
        let w = spawn(&types, "minecraft:wither_skeleton", 1);
        assert_eq!(value(&w, Attr::AttackDamage), 4.0);
        assert_eq!(w.attrs.attack_damage, 4.0);
    }
    /// #2783: runtime spawns run `Mob.finalizeSpawn`: the follow range
    /// bonus and 5 percent left-handed mobs.
    #[test]
    fn runtime_spawns_run_the_mob_finalize_spawn() {
        let Some(types) = types() else { return };
        let mut left = 0;
        for id in 1..=400 {
            let m = spawn(&types, "minecraft:skeleton", id);
            assert!(has_modifier(
                &m,
                Attr::FollowRange,
                "minecraft:random_spawn_bonus"
            ));
            left += usize::from(m.extra.left_handed);
        }
        // 5 percent of 400 is 20.
        assert!((5..=40).contains(&left), "{left} of 400 left-handed");
        // A bred baby never ran it.
        let mut cow = spawn(&types, "minecraft:cow", 1);
        cow.extra.left_handed = true;
        bred_offspring(&mut cow);
        assert!(!has_modifier(
            &cow,
            Attr::FollowRange,
            "minecraft:random_spawn_bonus"
        ));
        assert!(!cow.extra.left_handed);
        assert_eq!(cow.attrs.follow_range, value(&cow, Attr::FollowRange));
        // CaveSpider.finalizeSpawn does not call super.
        let c = spawn(&types, "minecraft:cave_spider", 1);
        assert!(!has_modifier(
            &c,
            Attr::FollowRange,
            "minecraft:random_spawn_bonus"
        ));
    }
    /// `Zombie` / `AbstractSkeleton` halloween garb: 25% of empty heads, never
    /// on other mobs, and not when it is not Halloween.
    #[test]
    fn halloween_pumpkins_only_undead_on_halloween() {
        let Some(types) = types() else { return };
        let mut pumpkins = 0;
        let mut lanterns = 0;
        for id in 1..=400 {
            let mut m = spawn(&types, "minecraft:skeleton", id);
            m.extra.set_slot_id(horizon_items::equipment::HEAD, None);
            let mut random = m.random.clone();
            crate::mobs::finalize::halloween_head(&mut m, &mut random, true);
            match m.extra.head_id() {
                Some("minecraft:jack_o_lantern") => {
                    pumpkins += 1;
                    lanterns += 1;
                    assert_eq!(crate::mobs::finalize::stored_drop_chance(&m, 5), 0.0);
                }
                Some("minecraft:carved_pumpkin") => {
                    pumpkins += 1;
                    assert_eq!(crate::mobs::finalize::stored_drop_chance(&m, 5), 0.0);
                }
                Some(other) => panic!("unexpected head {other}"),
                None => {}
            }
        }
        assert!(
            (70..=130).contains(&pumpkins),
            "{pumpkins} of 400 skeletons"
        );
        assert!(lanterns < pumpkins / 2, "{lanterns} lanterns of {pumpkins}");
        let mut m = spawn(&types, "minecraft:skeleton", 1);
        m.extra.set_slot_id(horizon_items::equipment::HEAD, None);
        let mut random = m.random.clone();
        crate::mobs::finalize::halloween_head(&mut m, &mut random, false);
        assert!(m.extra.head_id().is_none());
        let mut cow = spawn(&types, "minecraft:cow", 1);
        cow.extra.set_slot_id(horizon_items::equipment::HEAD, None);
        let mut random = cow.random.clone();
        crate::mobs::finalize::halloween_head(&mut cow, &mut random, true);
        assert!(cow.extra.head_id().is_none());
    }
}
