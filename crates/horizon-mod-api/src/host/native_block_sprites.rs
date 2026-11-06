//! The `native_block_sprites` area: host methods the game implements (`game/host/native_block_sprites.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `native_block_sprites` area, implemented by the game and reached through [`ClientHost`].
pub trait NativeBlockSpritesClientHost {
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// For a client mod: what each block (`minecraft:grass_block`, `oak_log[axis=x]`; properties left out take the
    /// block's default) draws on each of its six sides, as the mesher bakes its model: each face's sprite (array and
    /// layer, as `WorldDrawn::block_sprites` cuts them), where it lies on the side, how its texture runs across it,
    /// its tint source and whether it is cut out, translucent or animated; its particle sprite; whether it is a plain
    /// cube. None for a block the game does not know. The model is the state's first random variant. API
    /// 380.
    fn block_sprites(&self, blocks: &[&str]) -> Vec<Option<BlockSprites>> {
        crate::phase::unprovided(
            "ClientHost::block_sprites",
            "block_sprites",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// For a client mod: each texture's sprite by name (`minecraft:block/stone`, `block/stone`), none for one no
    /// model uses. API 380.
    fn texture_sprites(&self, textures: &[&str]) -> Vec<Option<SpriteRef>> {
        crate::phase::unprovided(
            "ClientHost::texture_sprites",
            "texture_sprites",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// For a client mod: where GUI sprite `sprite` lies in the GUI atlas and its size, none for one the atlas lacks
    /// (a server's metrics-only atlas lacks them all). The atlas names vanilla's sprites by path alone
    /// (`gui/sprites/container/slot`) and a mod's with its namespace (`mymod:gui/panel`).
    fn gui_sprite(&self, sprite: &Identifier) -> Option<AtlasRegion> {
        crate::phase::unprovided(
            "ClientHost::gui_sprite",
            "gui_sprite",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// For a client mod: changes each time the resources load (a resource pack turned on or off can move every
    /// sprite): ask `native_block_sprites` again then. API 380.
    fn block_sprites_version(&self) -> u64 {
        crate::phase::unprovided(
            "ClientHost::block_sprites_version",
            "block_sprites_version",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
}
