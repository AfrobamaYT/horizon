//! The `crosshair_target` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::pick_entity::Game<'_>::native_crosshair_target"
    )]
    fn crosshair_target();
}
