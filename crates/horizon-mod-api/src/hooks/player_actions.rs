//! The `player_actions` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_perform_action"
    )]
    fn player_perform_action();
}
