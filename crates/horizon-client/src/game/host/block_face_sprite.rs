//! The game's side of the `block_face_sprite` area (`horizon-mod-api/src/host/block_face_sprite.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl BlockFaceSpriteClientHost for Game<'_> {
    fn block_face_sprite(
        &self,
        state: BlockStateId,
        side: Direction,
    ) -> Option<BlockFaceTexture> {
        mod_api::__private::check_access(
            "block_face_sprite",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_block_face_sprite(state, side)
    }
}
