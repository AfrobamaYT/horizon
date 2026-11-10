//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct SpawnEntry {
    pub entity: String,
    pub weight: u32,
    pub min: u32,
    pub max: u32,
    /// A `ConstantInt` count (sampling draws nothing).
    pub constant: bool,
}
/// A biome's colours as a mod may change them (`native_biome_colors`), each
/// 0xRRGGBB: what tints grass (before the dark forest's modifier; the swamp's
/// noise colours replace it), leaves and vines, dry foliage (leaf litter) and
/// water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct BiomeColors {
    pub grass: u32,
    pub foliage: u32,
    pub dry_foliage: u32,
    pub water: u32,
}
/// One spawn category of a biome's natural spawns (`MobSpawnSettings`): what
/// `level_biome_spawners` reads.
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct BiomeSpawners {
    /// The category (`monster`, `creature`, `ambient`, `axolotls`, `water_creature`...).
    pub category: String,
    /// The category's spawners, in the biome file's order.
    pub entries: Vec<SpawnEntry>,
}
/// A biome's own sky, fog and water colours, each 0xRRGGBB, as its file declares them: what
/// `level_biome_effects` reads. `None` is a colour the biome leaves to the dimension's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct BiomeEffects {
    /// `minecraft:visual/sky_color`.
    pub sky: Option<u32>,
    /// `minecraft:visual/fog_color`.
    pub fog: Option<u32>,
    /// `effects.water_color`.
    pub water: u32,
    /// `minecraft:visual/water_fog_color`.
    pub water_fog: u32,
}
