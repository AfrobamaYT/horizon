//! The `sound_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_sound_refs::Game<'_>::native_player_stop_sound"
    )]
    fn player_stop_sound();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_sound_refs::Game<'_>::native_player_play_sound"
    )]
    fn player_play_sound();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_sound_refs::Game<'_>::native_entity_play_bound_sound"
    )]
    fn entity_play_bound_sound();
}
