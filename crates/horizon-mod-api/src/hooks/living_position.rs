//! The `living_position` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_position::Game<'_>::native_living_has_line_of_sight"
    )]
    fn living_has_line_of_sight();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_position::Game<'_>::native_living_random_teleport"
    )]
    fn living_random_teleport();
}
