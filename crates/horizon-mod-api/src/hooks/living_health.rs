//! The `living_health` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_health"
    )]
    fn living_health();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_max_health"
    )]
    fn living_max_health();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_alive"
    )]
    fn living_alive();
}
