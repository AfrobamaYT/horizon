//! The `level_craft` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_recipes::Game<'_>::native_level_craft"
    )]
    fn level_craft();
}
