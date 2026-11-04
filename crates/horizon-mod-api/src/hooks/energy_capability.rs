//! The `energy_capability` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_energy::Game<'_>::native_energy_amount"
    )]
    fn energy_amount();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_energy::Game<'_>::native_energy_capacity"
    )]
    fn energy_capacity();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_energy::Game<'_>::native_energy_insert"
    )]
    fn energy_insert();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_energy::Game<'_>::native_energy_extract"
    )]
    fn energy_extract();
}
