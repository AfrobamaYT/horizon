//! The `level_events` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_events::Game<'_>::native_level_level_event"
    )]
    fn level_level_event();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_events::Game<'_>::native_level_game_event"
    )]
    fn level_game_event();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_events::Game<'_>::native_level_explode"
    )]
    fn level_explode();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_events::Game<'_>::native_level_strike_lightning"
    )]
    fn level_strike_lightning();
}
