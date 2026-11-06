//! The `chunk_forced` area: host methods the game implements (`game/host/chunk_forced.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `chunk_forced` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ChunkForcedBackend {
    /// Side: server.
    /// A handle for chunk `pos` of an open level, loaded or not, so a mod can
    /// force an unloaded chunk. None for a stale handle, on a client, and for
    /// a chunk outside the world (`ChunkPos` past 30 million blocks).
    fn level_chunk_ref(&self, level: LevelRef, pos: [i32; 2]) -> Option<ChunkRef> {
        crate::phase::unprovided(
            "Backend::level_chunk_ref",
            "level_chunk_ref",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether `/forceload` holds the chunk (`ServerLevel.getForcedChunks`),
    /// loaded or not; a mod's own ticket on it does not count. None for a stale handle or on a client.
    fn chunk_forced(&self, chunk: ChunkRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::chunk_forced",
            "chunk_forced",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The level's force-loaded chunks (`/forceload query`), sorted as
    /// the command lists them. None for a stale handle or on a client.
    fn level_forced_chunks(&self, level: LevelRef) -> Option<Vec<[i32; 2]>> {
        crate::phase::unprovided(
            "Backend::level_forced_chunks",
            "level_forced_chunks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The chunk tickets `controller` holds in the level, as (owner, chunk
    /// position, natural_spawning), in order. None for a stale handle, on a
    /// client, outside a mod's hook and for another mod's controller.
    fn level_mod_forced_chunks(
        &self,
        level: LevelRef,
        controller: TicketController,
    ) -> Option<Vec<(NativeTicketOwner, [i32; 2], bool)>> {
        crate::phase::unprovided(
            "Backend::level_mod_forced_chunks",
            "level_mod_forced_chunks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `chunk_forced` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ChunkForcedBackendMut {
    /// Side: server.
    /// `/forceload add` or `remove` of one chunk (`ServerLevel.setChunkForced`):
    /// true when the forced set changed. A forced chunk is kept loaded and
    /// entity ticking, saved with the level, and the level keeps ticking
    /// without a player. This is the world's own `/forceload` state, not a
    /// ticket the mod owns: the chunk stays forced across restarts, and after
    /// the mod is unloaded or removed, until `set_forced(false)` or
    /// `/forceload remove` clears it. Unlike vanilla, the chunk is not loaded before
    /// this returns: it loads over the next ticks, as after `/forceload`.
    /// False for a stale handle, on a client, and for a level the server
    /// has not opened.
    fn chunk_set_forced(&mut self, chunk: ChunkRef, forced: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::chunk_set_forced",
            "chunk_set_forced",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The chunk ticket of `controller` for `owner` on `chunk`
    /// (`TicketController.forceChunk`): true when the controller's tickets
    /// changed. The controller must be the calling mod's own.
    /// A chunk stays forced, entity ticking and saved, and its level keeps
    /// ticking, while any ticket or `/forceload` holds it. With
    /// `natural_spawning` the chunk joins the spawning chunks (inhabited
    /// time, thunder, a spawn attempt) while it lies in the spawning area of a
    /// player (`ChunkMap.collectSpawningChunks`); the local mob cap still lets
    /// mobs spawn only with a non-spectator within 128 blocks. One
    /// owner holds a plain and a natural-spawning ticket on a chunk apart. Tickets are
    /// scoped by (controller, owner, chunk), so two owners in one chunk, or
    /// two controllers, release separately. They are saved in
    /// `data/horizon/mod_chunk_tickets.dat` under the controller's id; when
    /// the world loads, those of a controller no loaded mod registers are
    /// discarded. False outside a mod's hook, for another mod's controller, on
    /// a client, for a chunk outside the world and for a level the server has
    /// not opened.
    fn chunk_set_mod_forced(
        &mut self,
        chunk: ChunkRef,
        controller: TicketController,
        owner: NativeTicketOwner,
        forced: bool,
        natural_spawning: bool,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::chunk_set_mod_forced",
            "chunk_set_mod_forced",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
