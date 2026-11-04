//! The `level_effects` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_effects::Game<'_>::native_level_play_sound"
    )]
    fn level_play_sound();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_effects::Game<'_>::native_level_send_particles"
    )]
    fn level_send_particles();
}
