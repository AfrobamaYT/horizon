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
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_kind"
    )]
    fn block_entity_kind();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_position"
    )]
    fn block_entity_position();
    #[helper(
        side = "server",
        receiver = "block_entity",
        target = "horizon_client::game::native_block_entity_refs::Game<'_>::native_block_entity_level"
    )]
    fn block_entity_level();
    #[helper(
        side = "server",
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
}
