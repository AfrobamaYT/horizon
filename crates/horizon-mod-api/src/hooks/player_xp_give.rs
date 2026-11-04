//! The `player_xp_give` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_give_experience_points"
    )]
    fn player_give_experience_points();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_give_levels"
    )]
    fn player_give_levels();
}
