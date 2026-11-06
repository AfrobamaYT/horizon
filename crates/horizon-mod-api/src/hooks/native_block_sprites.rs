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
    #[helper(
        side = "client",
        resource_read = true,
        target = "horizon_client::game::native_block_sprites::Game<'_>::native_block_sprites_version"
    )]
    fn block_sprites_version();
}
