//! The `chunk_refs` area: host methods the game implements (`game/host/chunk_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `chunk_refs` area, implemented by the game and reached through [`Host`].
pub trait ChunkRefsHost {
    /// Side: both.
    /// Find a loaded chunk in an open server level.
    fn chunk(&self, level: LevelRef, pos: [i32; 2]) -> Option<ChunkRef> {
        crate::phase::unprovided("Host::chunk", "chunk", crate::phase::LoadAccess::WorldOnly)
    }
}
/// The handles' reads of the `chunk_refs` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ChunkRefsBackend {
    /// Side: server.
    /// Whether this chunk is loaded; false on a client or for a stale handle.
    fn chunk_loaded(&self, chunk: ChunkRef) -> bool {
        crate::phase::unprovided(
            "Backend::chunk_loaded",
            "chunk_loaded",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The status of a loaded chunk. Proto chunks are not exposed here.
    fn chunk_status(&self, chunk: ChunkRef) -> Option<ChunkStatus> {
        crate::phase::unprovided(
            "Backend::chunk_status",
            "chunk_status",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
