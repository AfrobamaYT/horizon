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
    /// Applies bone meal to the block at `pos` as a dispenser does, with no player and no item
    /// (`DispenserBlock`'s `BoneMealItem.growCrop` then `growWaterPlant`): the mods' `bonemeal`
    /// hook is asked first and a canceled one's answer stands, then the block's own
    /// `isValidBonemealTarget`, `isBonemealSuccess` and `performBonemeal` decide and run, the
    /// level's random draws as vanilla's, and level event 1505 (the growth particles and sound)
    /// follows. True when something grew. False for a block that is no target, a failed roll, an
    /// unloaded or out-of-height position, a stale or parked handle, and off a server.
    #[notes = r#"## Bone meal at a position

`LevelRef::bone_meal(host, pos)` runs the whole bone meal sequence on one block for a mod that
grows things itself (a sprinkler, a growth totem), so it neither reimplements the checks nor
skips `bonemeal` listeners. It takes no stack: nothing is consumed, and a hook that edits the
stack it is shown edits a throwaway one."#]
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_bone_meal"
    )]
    fn level_bone_meal();
    /// Sets the biome of the 4x4x4 cell holding `pos` to `biome` (`minecraft:plains`), as
    /// `/fillbiome` does for one cell: the chunk is saved, re-meshed and the players who see it
    /// are sent the new biome. False for an unknown biome, an unloaded or out-of-height position,
    /// a cell that already has it, a stale or parked handle, and off a server.
    #[notes = r#"## Setting a biome

`LevelRef::set_biome(host, pos, biome)` changes one biome cell, the 4x4x4 block volume a biome is
stored in, so every position in that cell changes with it. It shares `/fillbiome`'s save, mesh and
resend path, so the change survives a reload and reaches players as the command's does. Name a
biome the world knows (a data pack's included); anything else is refused."#]
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_set_biome"
    )]
    fn level_set_biome();
}
