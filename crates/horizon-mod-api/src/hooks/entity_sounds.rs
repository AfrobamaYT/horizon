//! The `entity_sounds` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_sounds::Game<'_>::native_entity_sound_source"
    )]
    fn entity_sound_source();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_sounds::Game<'_>::native_entity_play_sound"
    )]
    fn entity_play_sound();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_sounds::Game<'_>::native_entity_game_event"
    )]
    fn entity_game_event();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_sounds::Game<'_>::native_entity_broadcast_event"
    )]
    fn entity_broadcast_event();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_sounds::Game<'_>::native_entity_spectator"
    )]
    fn entity_spectator();
}
