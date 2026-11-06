//! The `native_block_sprites` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_block_sprites"
    )]
    fn block_sprites();
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_texture_sprites"
    )]
    fn texture_sprites();
    /// Where GUI sprite `sprite` lies in the GUI atlas (`minecraft:gui/sprites/container/slot`, a
    /// particle's `minecraft:particle/flame`, a content mod's `mymod:gui/panel`) and its size,
    /// none for a sprite the atlas lacks. A snapshot of the atlas built last: read it in
    /// `texture_atlas_stitched` for `TextureAtlas::Gui` and keep it until the next one.
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_gui_sprite"
    )]
    fn gui_sprite();
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_block_sprites_version"
    )]
    fn block_sprites_version();
}
