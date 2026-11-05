//! The `level_block_writes` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_set_block"
    )]
    fn level_set_block();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_remove_block"
    )]
    fn level_remove_block();
    /// Breaks the block at `pos` as a machine does, with no player: vanilla's
    /// `Level.destroyBlock(pos, true)` with the loot rolled for `tool`
    /// (`Block.dropResources(state, level, pos, blockEntity, null, tool)`):
    /// the break particles and sound, the block's drops with what its block
    /// entity copies into them (a shulker box's contents), experience for an
    /// ore as with no tool, then the block's fluid in its place. `tool` picks
    /// the drops as a player's held item does (silk touch, fortune, shears);
    /// `None` is an empty hand. The mods' say on drops (`native_loot_drops`)
    /// applies. With `collect`, the drops are returned and nothing is popped
    /// (a harvester storing them); without, they pop as item entities and the
    /// list is empty. Nothing drops where `doTileDrops` is off. None, with
    /// nothing broken, for air, an unloaded block and off a server.
    /// API @API:machine_breaking@.
    /// Diverges from upstream: this extends `LevelRef::destroy_block`; `drop` false
    /// suppresses resources, and stale/parked handles and out-of-height positions refuse.
    /// The canonical block-drop hook runs once, before collection or spawning.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_destroy_block"
    )]
    fn level_destroy_block();
    /// `ServerLevel.destroyBlockProgress`: shows breaker `breaker`'s crack on
    /// the block at `pos` at stage `progress` (0 to 9) to the players within 64
    /// blocks, as a zombie breaking a door shows its; any other stage takes the
    /// crack away. A machine passes an id of its own that no entity has (a
    /// negative one), the same each time, so a new stage replaces its old one;
    /// a crack not renewed for 400 ticks fades. Off a server nothing happens.
    /// API @API:machine_breaking@.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_set_break_progress"
    )]
    fn level_set_break_progress();
}
