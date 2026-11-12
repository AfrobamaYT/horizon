//! The `client_chunk_load` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Chunk `chunk` (`[x, z]`, in chunks) of `dimension` came into this
    /// client's level from the server (NeoForge's `ChunkEvent.Load` on the
    /// client, whose `isNewChunk` is always false there): once per chunk
    /// packet the client adopts, after the column and its block entities are
    /// in the level, so a listener reads its blocks through `ClientHost`. A
    /// column the server sends again while it is resident is told again.
    /// Only on a client of a server, a LAN guest included; an integrated
    /// world's own chunks have the server's `native_chunk_load`. Client-side.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "ChunkEvent.Load",
        cancellable = false
    )]
    fn client_chunk_load();
}
