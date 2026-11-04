//! The `fluid_capability` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_fluids::Game<'_>::native_fluid_tank_count"
    )]
    fn fluid_tank_count();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_fluids::Game<'_>::native_fluid_tank"
    )]
    fn fluid_tank();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_fluids::Game<'_>::native_fluid_tank_capacity"
    )]
    fn fluid_tank_capacity();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_fluids::Game<'_>::native_fluid_tank_insert"
    )]
    fn fluid_tank_insert();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_fluids::Game<'_>::native_fluid_tank_extract"
    )]
    fn fluid_tank_extract();
}
