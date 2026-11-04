//! The `level_shapes` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_block_shape"
    )]
    fn level_block_shape();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_clip"
    )]
    fn level_clip();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level_block_face_sturdy"
    )]
    fn level_block_face_sturdy();
}
