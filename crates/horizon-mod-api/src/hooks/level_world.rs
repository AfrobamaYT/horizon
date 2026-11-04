//! The `level_world` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_seed"
    )]
    fn level_seed();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_difficulty"
    )]
    fn level_difficulty();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_game_rule"
    )]
    fn level_game_rule();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_weather"
    )]
    fn level_weather();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_is_raining"
    )]
    fn level_is_raining();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_is_raining_at"
    )]
    fn level_is_raining_at();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_enabled_features"
    )]
    fn level_enabled_features();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_world::Game<'_>::native_level_is_thundering"
    )]
    fn level_is_thundering();
    #[helper(side = "server", receiver = "level")]
    fn level_random();
    #[helper(side = "server", receiver = "level")]
    fn level_set_random();
}
