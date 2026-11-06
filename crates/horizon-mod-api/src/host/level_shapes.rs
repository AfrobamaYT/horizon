//! The `level_shapes` area: host methods the game implements (`game/host/level_shapes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_shapes` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LevelShapesBackend {
    /// Side: server.
    /// Block-local outline or collision boxes with an empty context. Interaction,
    /// visual, occlusion and support shapes are not exposed. Returns None on a
    /// client, for a stale handle, or for a closed level. Unloaded blocks read
    /// as air; collision errors return no boxes, like `native_block_collision`.
    fn level_block_shape(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        kind: NativeBlockShapeKind,
    ) -> Option<Vec<[f64; 6]>> {
        crate::phase::unprovided(
            "Backend::level_block_shape",
            "level_block_shape",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Clip an empty-context ray against collider or outline shapes, without fluids.
    /// None on failed reads, dynamic shapes, invalid rays or traversal limits.
    fn level_clip(
        &self,
        level: LevelRef,
        from: [f64; 3],
        to: [f64; 3],
        blocks: NativeClipBlock,
        fluids: NativeClipFluid,
    ) -> Option<NativeClipHit> {
        crate::phase::unprovided(
            "Backend::level_clip",
            "level_clip",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Gameplay's face support in the current or a parked server level, including
    /// live block entities for dynamic shapes. None for stale or closed levels,
    /// unloaded blocks, unsupported states or a shape query error.
    fn level_block_face_sturdy(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        face: NativeBlockFace,
        support: NativeSupportType,
    ) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::level_block_face_sturdy",
            "level_block_face_sturdy",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
