//! The `living_health_writes` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_set_health"
    )]
    fn living_set_health();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_heal"
    )]
    fn living_heal();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_kill_credit"
    )]
    fn living_kill_credit();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_last_attacker"
    )]
    fn living_last_attacker();
}
