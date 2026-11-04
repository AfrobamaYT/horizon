//! The `entity_fall_and_push` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_reset_fall_distance"
    )]
    fn entity_reset_fall_distance();
    #[helper(
        side = "server",
        receiver = "minecart",
        target = "horizon_client::game::native_minecart_refs::Game<'_>::native_minecart_push"
    )]
    fn minecart_push();
    #[helper(
        side = "server",
        receiver = "minecart",
        target = "horizon_client::game::native_minecart_refs::Game<'_>::native_minecart_set_push"
    )]
    fn minecart_set_push();
}
