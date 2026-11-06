//! The `level_signals` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Weak power at a block in the given direction, including conducted power.
    /// Reads the current server level; returns None for a stale, client or other-level handle.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_signal"
    )]
    fn level_signal();
    /// Strong power at a block in the given direction, or None if the level is unavailable.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_direct_signal"
    )]
    fn level_direct_signal();
    /// Strongest direct signal received by a block, or None if the level is unavailable.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_direct_signal_to"
    )]
    fn level_direct_signal_to();
    /// Strongest signal received from neighbours, or None if the level is unavailable.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_best_neighbor_signal"
    )]
    fn level_best_neighbor_signal();
    /// Whether any neighbour supplies a signal; false if the level is unavailable.
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_has_neighbor_signal"
    )]
    fn level_has_neighbor_signal();
}
