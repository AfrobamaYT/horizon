//! The `chunk_forced` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_chunk_ref"
    )]
    fn level_chunk_ref();
    #[helper(
        side = "server",
        receiver = "chunk",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk_forced"
    )]
    fn chunk_forced();
    #[helper(
        side = "server",
        receiver = "chunk",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk_set_forced"
    )]
    fn chunk_set_forced();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_forced_chunks"
    )]
    fn level_forced_chunks();
    #[helper(
        side = "server",
        receiver = "chunk",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_chunk_set_mod_forced"
    )]
    fn chunk_set_mod_forced();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_mod_forced_chunks"
    )]
    fn level_mod_forced_chunks();
}
