//! The `block_entity_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_ref"
    )]
    fn block_entity();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_player_open_block_menu"
    )]
    fn player_open_menu();
    #[helper(
        side = "both",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_kind"
    )]
    fn block_entity_kind();
    #[helper(
        side = "both",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_position"
    )]
    fn block_entity_position();
    #[helper(
        side = "both",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_level"
    )]
    fn block_entity_level();
    #[helper(
        side = "both",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_data"
    )]
    fn block_entity_data();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_merge_data"
    )]
    fn block_entity_merge_data();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_replace_data"
    )]
    fn block_entity_replace_data();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_set_changed"
    )]
    fn block_entity_set_changed();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_name"
    )]
    fn block_entity_name();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_set_name"
    )]
    fn block_entity_set_name();
    /// Read a snapshot of a mod block entity's menu data slots, not its item storage.
    /// Reads the server's entity or the client's own copy; None for vanilla/missing,
    /// stale or unloaded entities. Access item storage through Parts.
    #[helper(
        side = "both",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_data_slots"
    )]
    fn block_entity_data_slots();
    /// Set a declared mod block-entity data slot on the server, without resizing it.
    /// Joins the current transaction, rolls back on abort, and calls setChanged at
    /// commit. False for an invalid/stale/client/parked entity, a missing slot, or
    /// no change. The normal menu broadcast carries changed values to its viewers.
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_set_data_slot"
    )]
    fn block_entity_set_data_slot();
}
