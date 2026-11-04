//! The `level_item_entities` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_items::Game<'_>::native_level_pop_resource"
    )]
    fn level_pop_resource();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_items::Game<'_>::native_level_spawn_item"
    )]
    fn level_spawn_item();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_items::Game<'_>::native_level_spawn_item_with_motion"
    )]
    fn level_spawn_item_with_motion();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_items::Game<'_>::native_level_award_experience"
    )]
    fn level_award_experience();
}
