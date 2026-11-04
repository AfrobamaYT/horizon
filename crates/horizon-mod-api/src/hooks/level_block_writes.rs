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
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_destroy_block"
    )]
    fn level_destroy_block();
}
