//! The `level_neighbor_updates` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_update_neighbors"
    )]
    fn level_update_neighbors();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_neighbor_changed"
    )]
    fn level_neighbor_changed();
}
