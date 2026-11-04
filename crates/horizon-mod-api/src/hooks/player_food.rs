//! The `player_food` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_needs_food"
    )]
    fn player_needs_food();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_has_enough_food"
    )]
    fn player_has_enough_food();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_can_eat"
    )]
    fn player_can_eat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_feed"
    )]
    fn player_feed();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_add_exhaustion"
    )]
    fn player_add_exhaustion();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_set_food_level"
    )]
    fn player_set_food_level();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_set_saturation"
    )]
    fn player_set_saturation();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_set_exhaustion"
    )]
    fn player_set_exhaustion();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_experience"
    )]
    fn player_experience();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_xp_needed_for_next_level"
    )]
    fn player_xp_needed_for_next_level();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_set_experience_points"
    )]
    fn player_set_experience_points();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_food::Game<'_>::native_player_set_experience_levels"
    )]
    fn player_set_experience_levels();
}
