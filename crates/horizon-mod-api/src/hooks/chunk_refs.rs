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
    /// Every loaded chunk of the level (`ServerChunkCache` loaded chunks), as handles ordered by
    /// chunk x then z. Server-side; None for a stale handle or on a client. Loads nothing, and the
    /// list is a snapshot: a chunk can unload after it, so read `chunk_loaded` again before use.
    #[notes = r#"## Loaded chunks

`LevelRef::loaded_chunks(host)` lists the chunks the level holds now, the same ones `chunk`
finds by position, ordered by x then z so two calls over an unchanged level agree. It loads no
chunk and sorts a copy of the level's loaded set, so call it when a mod needs the whole set, not
for a position it already knows."#]
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_level_refs::Game<'_>::native_level_loaded_chunks"
    )]
    fn level_loaded_chunks();
}
