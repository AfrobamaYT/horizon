//! The `level_reads` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_state"
    )]
    fn level_block_state();
    /// Preview a tool transformation in this level with the existing block-tool hook marked simulated.
    /// Does not apply the result; invalid, unloaded or unavailable contexts return None.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_block_transform"
    )]
    fn level_block_transform();
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_block"
    )]
    fn level_block();
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_block_property"
    )]
    fn level_block_property();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_in_place"
    )]
    fn level_in_place();
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_is_loaded"
    )]
    fn level_is_loaded();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_brightness"
    )]
    fn level_brightness();
    /// The level's lowest block y and height (`LevelHeightAccessor.getMinY`, `getHeight`).
    /// Reads any loaded server level or the client's current level; None for a stale
    /// handle, before client login, or for a dimension unavailable on this side.
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_height"
    )]
    fn level_height();
    /// Whether `pos` is inside the level's world bounds (`Level.isInWorldBounds`): within 30,000,000
    /// blocks of the origin on x and z, and between the level's lowest and highest build height.
    /// None for a stale handle, before client login, or for a dimension unavailable on this side.
    /// Does not load chunks.
    #[notes = r#"## World and spawnable bounds

`LevelRef::in_world_bounds(host, pos)` and `in_spawnable_bounds(host, pos)` answer
`Level.isInWorldBounds` and `Level.isInSpawnableBounds` without loading a chunk. The first also
needs the position inside the level's build height; the second is the wider limit `/summon` and
`/tp` use (30,000,000 on x and z, 20,000,000 on y, whatever the level's height). A mod checks them
before reading or writing a far position, rather than learning it from a failed write."#]
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_in_world_bounds"
    )]
    fn level_in_world_bounds();
    /// Whether `pos` is inside the spawnable bounds (`Level.isInSpawnableBounds`): within
    /// 30,000,000 blocks on x and z and 20,000,000 on y. None as for `in_world_bounds`.
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_in_spawnable_bounds"
    )]
    fn level_in_spawnable_bounds();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_game_time"
    )]
    fn level_game_time();
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_day_time"
    )]
    fn level_day_time();
    #[helper(
        side = "both",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_fluid_flow"
    )]
    fn level_fluid_flow();
}
