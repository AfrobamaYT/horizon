//! The `block_face_sprite` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// For a client mod drawing a block entity: the texture block state `state`'s model shows on its `side`
    /// (`block/oak_planks`: vanilla's without their namespace, a mod's with it), by the name `native_draw_cube` and
    /// `native_draw_quad` take, and that face's
    /// tint index (-1 for none): a copycat block drawing a box of its material's textures with `native_draw_cube`,
    /// kept with `BlockEntityDraw::keep`. The face culled against that side comes first, else the first facing it;
    /// None when the model has no face that way or isn't baked. API @API:block_face_sprite@.
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_block_face_sprite"
    )]
    fn block_face_sprite();
}
