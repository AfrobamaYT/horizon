//! The `player_death_location` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_death::Game<'_>::native_player_last_death_location"
    )]
    fn player_last_death_location();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_death::Game<'_>::native_player_set_last_death_location"
    )]
    fn player_set_last_death_location();
}
