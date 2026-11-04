//! The `chunk_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk"
    )]
    fn chunk();
    #[helper(
        side = "server",
        receiver = "chunk",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk_ref_loaded"
    )]
    fn chunk_loaded();
    #[helper(
        side = "server",
        receiver = "chunk",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk_status"
    )]
    fn chunk_status();
}
