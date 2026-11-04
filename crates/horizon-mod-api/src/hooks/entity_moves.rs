//! The `entity_moves` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_teleport_to"
    )]
    fn entity_teleport_to();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_set_rotation"
    )]
    fn entity_set_rotation();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_head_yaw"
    )]
    fn entity_head_yaw();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_set_head_yaw"
    )]
    fn entity_set_head_yaw();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_direction"
    )]
    fn entity_direction();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_dimensions"
    )]
    fn entity_dimensions();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_moves::Game<'_>::native_entity_push"
    )]
    fn entity_push();
}
