//! The game's side of the `native_block_sprites` area (`horizon-mod-api/src/host/native_block_sprites.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl NativeBlockSpritesClientHost for Game<'_> {
    fn block_sprites(&self, blocks: &[&str]) -> Vec<Option<BlockSprites>> {
        mod_api::__private::check_access(
            "block_sprites",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_block_sprites(blocks)
    }
    fn texture_sprites(&self, textures: &[&str]) -> Vec<Option<SpriteRef>> {
        mod_api::__private::check_access(
            "texture_sprites",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_texture_sprites(textures)
    }
    fn gui_sprite(&self, sprite: &Identifier) -> Option<AtlasRegion> {
        mod_api::__private::check_access(
            "gui_sprite",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_gui_sprite(sprite)
    }
    fn block_sprites_version(&self) -> u64 {
        mod_api::__private::check_access(
            "block_sprites_version",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_block_sprites_version()
    }
}
