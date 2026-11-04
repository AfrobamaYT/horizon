//! The `player_abilities_write` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_set_abilities"
    )]
    fn player_set_abilities();
}
