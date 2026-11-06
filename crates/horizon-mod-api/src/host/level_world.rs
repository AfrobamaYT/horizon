//! The `level_world` area: host methods the game implements (`game/host/level_world.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_world` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelWorldBackend {
    /// Side: both.
    /// Dimension shape of a loaded server level, including parked levels.
    fn level_dimension_shape(&self, level: LevelRef) -> Option<LevelDimensionShape> {
        crate::phase::unprovided(
            "Backend::level_dimension_shape",
            "level_dimension_shape",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The world's seed (`WorldOptions.seed`), as `/seed` shows it.
    fn level_seed(&self, level: LevelRef) -> Option<i64> {
        crate::phase::unprovided(
            "Backend::level_seed",
            "level_seed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The difficulty (`Level.getDifficulty`): `peaceful`, `easy`, `normal`
    /// or `hard`, as `/difficulty` names it.
    fn level_difficulty(&self, level: LevelRef) -> Option<&'static str> {
        crate::phase::unprovided(
            "Backend::level_difficulty",
            "level_difficulty",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Read a registered boolean game rule.
    fn level_game_rule_bool(&self, level: LevelRef, key: BoolGameRuleKey) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::level_game_rule_bool",
            "level_game_rule_bool",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Game rule `rule` (`minecraft:keep_inventory` or `keep_inventory`), as
    /// `/gamerule` queries it: None for an unknown rule.
    fn level_game_rule(&self, level: LevelRef, rule: &str) -> Option<GameRuleValue> {
        crate::phase::unprovided(
            "Backend::level_game_rule",
            "level_game_rule",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Every game rule the world has (`GameRules`) with its current value, by `minecraft:` id
    /// (`max_minecart_speed` only with minecart improvements): None for a stale handle and on
    /// a client.
    fn level_game_rules(&self, level: LevelRef) -> Option<Vec<(String, GameRuleValue)>> {
        crate::phase::unprovided(
            "Backend::level_game_rules",
            "level_game_rules",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The rain and thunder levels, each 0.0 to 1.0 (`Level.getRainLevel`,
    /// `getThunderLevel` at partial tick 1.0): both 0 in a level with no weather.
    /// Reads any open server level or the client's current synchronized level;
    /// None for a stale handle, an unavailable level or a client not logged in.
    fn level_weather(&self, level: LevelRef) -> Option<(f32, f32)> {
        crate::phase::unprovided(
            "Backend::level_weather",
            "level_weather",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether it rains (`Level.isRaining`: a rain level above 0.2).
    fn level_is_raining(&self, level: LevelRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::level_is_raining",
            "level_is_raining",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Level.isRainingAt(pos)` in the level a handle names, the one in place or a parked one:
    /// it rains there and `pos` is under open sky in a biome that rains. None for a stale
    /// handle, on a client, and where `pos` is not loaded or outside the world's height; a
    /// read never loads a chunk.
    fn level_is_raining_at(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::level_is_raining_at",
            "level_is_raining_at",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LevelReader.enabledFeatures` of the level a handle names. Every level of a world shares
    /// the world's flags. None for a stale handle and on a client.
    fn level_enabled_features(&self, level: LevelRef) -> Option<FeatureFlags> {
        crate::phase::unprovided(
            "Backend::level_enabled_features",
            "level_enabled_features",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether it thunders (`Level.isThundering`: a thunder level above 0.9).
    fn level_is_thundering(&self, level: LevelRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::level_is_thundering",
            "level_is_thundering",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The random of the server level `level` names: vanilla's `Level.random`
    /// (`level.getRandom()`), the one its explosions, block drops and other
    /// level draws use, so a mod's draws move vanilla's later ones as a Java
    /// mod's do. A parked level (the Nether while the server ticks the
    /// Overworld) answers with its own stream. This is a copy of it as it
    /// stands: draw with `LevelRef::with_random`, which stores the stream
    /// back, so the draws move the level's own. `None` on a client of a
    /// server, for a stale handle and for a level not open: the Nether and
    /// the End open when someone first goes there and close after 300 ticks
    /// with nobody in them (`close_idle_levels`).
    fn level_random(&self, level: LevelRef) -> Option<LegacyRandom> {
        crate::phase::unprovided(
            "Backend::level_random",
            "level_random",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `level_world` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelWorldBackendMut {
    /// Side: server.
    /// Replace the random of the server level `level` names with `random`,
    /// a stream `native_level_random` gave and the mod drew from. False where
    /// that gives `None`.
    fn level_set_random(&mut self, level: LevelRef, random: LegacyRandom) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_set_random",
            "level_set_random",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
