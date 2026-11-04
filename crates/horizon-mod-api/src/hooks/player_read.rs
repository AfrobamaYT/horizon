//! The `player_read` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_server::Game<'_>::native_player_ref"
    )]
    fn player();
    #[helper(
        side = "server",
        target = "horizon_client::game::native_server::Game<'_>::native_fake_player"
    )]
    fn fake_player();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_is_fake"
    )]
    fn player_is_fake();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_remove_fake"
    )]
    fn player_remove_fake();
    #[helper(side = "both", receiver = "player")]
    fn player_inventory();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_game_mode"
    )]
    fn player_game_mode();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_is_creative"
    )]
    fn player_is_creative();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_is_spectator"
    )]
    fn player_is_spectator();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_may_build"
    )]
    fn player_may_build();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_food"
    )]
    fn player_food();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_abilities"
    )]
    fn player_abilities();
}
