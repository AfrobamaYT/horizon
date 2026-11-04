//! The `entity_riding` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_vehicle"
    )]
    fn entity_vehicle();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_is_passenger"
    )]
    fn entity_is_passenger();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_root_vehicle"
    )]
    fn entity_root_vehicle();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_passengers"
    )]
    fn entity_passengers();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_is_vehicle"
    )]
    fn entity_is_vehicle();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_start_riding"
    )]
    fn entity_start_riding();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_stop_riding"
    )]
    fn entity_stop_riding();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_riding::Game<'_>::native_entity_eject_passengers"
    )]
    fn entity_eject_passengers();
}
