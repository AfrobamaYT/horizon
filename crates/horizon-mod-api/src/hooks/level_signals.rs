//! The `level_signals` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_signal"
    )]
    fn level_signal();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_direct_signal"
    )]
    fn level_direct_signal();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_direct_signal_to"
    )]
    fn level_direct_signal_to();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_best_neighbor_signal"
    )]
    fn level_best_neighbor_signal();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_has_neighbor_signal"
    )]
    fn level_has_neighbor_signal();
}
