//! The game's side of the `client_render` area (`horizon-mod-api/src/host/client_render.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientRenderClientHost for Game<'_> {
    fn refresh_biome_colors(&mut self) {
        mod_api::__private::check_access(
            "refresh_biome_colors",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_refresh_biome_colors()
    }
    fn mod_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> bool {
        mod_api::__private::check_access("mod_image", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mod_image(width, height, rgba)
    }
    fn block_face_sprite(&self, state: BlockStateId, side: Direction) -> Option<BlockFaceTexture> {
        mod_api::__private::check_access(
            "block_face_sprite",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_block_face_sprite(state, side)
    }
    fn refresh_render_visibility(&mut self) {
        mod_api::__private::check_access(
            "refresh_render_visibility",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_refresh_render_visibility()
    }
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
