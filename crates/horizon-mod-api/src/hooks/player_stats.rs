//! The `player_stats` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_stat"
    )]
    fn player_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_award_stat"
    )]
    fn player_award_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_reset_stat"
    )]
    fn player_reset_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_has_discovered_recipe"
    )]
    fn player_has_discovered_recipe();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_award_recipes"
    )]
    fn player_award_recipes();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_reset_recipes"
    )]
    fn player_reset_recipes();
}
