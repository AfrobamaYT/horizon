//! The `level_block_writes` area: host methods the game implements (`game/host/level_block_writes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `level_block_writes` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelBlockWritesBackendMut {
    /// Side: server.
    /// Set a block in the level in place (`Level.setBlock`). Parked levels,
    /// stale handles, clients and unloaded or out-of-height positions return
    /// false. Client notification follows the supplied update flags.
    fn level_set_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        state: BlockStateId,
        flags: NativeBlockUpdateFlags,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_set_block",
            "level_set_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Remove a block in place (`Level.removeBlock`), preserving its fluid.
    fn level_remove_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        moved: bool,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_remove_block",
            "level_remove_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Destroy a block in the level in place (`Level.destroyBlock`), optionally dropping
    /// resources with `tool` (None is an empty hand). With `collect`, return the dropped
    /// stacks without spawning their item entities; otherwise return an empty list after
    /// spawning them. The one level drop path runs loot, block-drop hooks and experience
    /// once, in vanilla's random order. `drop` false suppresses resources entirely.
    /// None means nothing broke: air, a refused write, a stale handle, a client, an
    /// unloaded/out-of-height position, or a parked level.
    fn level_destroy_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        drop: bool,
        tool: Option<&ItemStack>,
        collect: bool,
    ) -> Option<Vec<ItemStack>> {
        crate::phase::unprovided(
            "BackendMut::level_destroy_block",
            "level_destroy_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Send `ServerLevel.destroyBlockProgress` in this level: stages 0..9 show a crack,
    /// any other stage removes it. A machine uses a stable negative breaker id.
    fn level_set_break_progress(
        &mut self,
        level: LevelRef,
        breaker: i32,
        pos: [i32; 3],
        progress: i32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_set_break_progress",
            "level_set_break_progress",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
