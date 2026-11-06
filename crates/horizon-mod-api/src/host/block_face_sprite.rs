//! The `block_face_sprite` area: host methods the game implements (`game/host/block_face_sprite.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `block_face_sprite` area, implemented by the game and reached through [`ClientHost`].
pub trait BlockFaceSpriteClientHost {
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// For a client mod drawing a block entity: the texture block state `state`'s model shows on its `side`
    /// (`block/oak_planks`: vanilla's without their namespace, a mod's with it), by the name `native_draw_cube` and
    /// `native_draw_quad` take, and that face's
    /// tint index (-1 for none): a copycat block drawing a box of its material's textures with `native_draw_cube`,
    /// kept with `BlockEntityDraw::keep`. The face culled against that side comes first, else the first facing it;
    /// None when the model has no face that way or isn't baked. API @API:block_face_sprite@.
    fn block_face_sprite(
        &self,
        state: BlockStateId,
        side: Direction,
    ) -> Option<BlockFaceTexture> {
        crate::phase::unprovided(
            "ClientHost::block_face_sprite",
            "block_face_sprite",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
}
