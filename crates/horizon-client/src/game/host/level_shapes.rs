//! The game's side of the `level_shapes` area (`horizon-mod-api/src/host/level_shapes.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl LevelShapesBackend for Game<'_> {
    fn level_block_shape(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        kind: NativeBlockShapeKind,
    ) -> Option<Vec<[f64; 6]>> {
        mod_api::__private::check_access(
            "level_block_shape",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_shape(level, pos, kind)
    }
    fn level_clip(
        &self,
        level: LevelRef,
        from: [f64; 3],
        to: [f64; 3],
        blocks: NativeClipBlock,
        fluids: NativeClipFluid,
    ) -> Option<NativeClipHit> {
        mod_api::__private::check_access("level_clip", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_clip(level, from, to, blocks, fluids)
    }
    fn level_block_face_sturdy(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        face: NativeBlockFace,
        support: NativeSupportType,
    ) -> Option<bool> {
        mod_api::__private::check_access(
            "level_block_face_sturdy",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_face_sturdy(level, pos, face, support)
    }
}
