//! The game's side of the `base` area (`horizon-mod-api/src/host/base.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use std::path::PathBuf;
use std::sync::Arc;
impl BaseHost for Game<'_> {
    fn attachments(&mut self) -> &mut dyn AttachmentStore {
        mod_api::__private::check_access("attachments", mod_api::__private::LoadAccess::WorldOnly);
        self.native_attachments()
    }
    fn lit_block_name(&self, pos: [i32; 3]) -> Option<&str> {
        mod_api::__private::check_access(
            "lit_block_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_lit_block_name(pos)
    }
    fn max_stack_size(&self, item: &str) -> Option<u32> {
        mod_api::__private::check_access(
            "max_stack_size",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_max_stack_size(item)
    }
    fn block_state_property(&self, state: BlockStateId, key: &str) -> Option<&str> {
        mod_api::__private::check_access(
            "block_state_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_property(state, key)
    }
    fn on(&self, mod_id: &str) -> bool {
        mod_api::__private::check_access("on", mod_api::__private::LoadAccess::WorldOnly);
        self.native_on(mod_id)
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        mod_api::__private::check_access("setting", mod_api::__private::LoadAccess::WorldOnly);
        self.native_setting(mod_id, key)
    }
    fn set_setting(&self, mod_id: &str, key: &str, value: &str) {
        mod_api::__private::check_access("set_setting", mod_api::__private::LoadAccess::WorldOnly);
        self.native_set_setting(mod_id, key, value)
    }
    fn is_loaded(&self, id: &str) -> bool {
        mod_api::__private::check_access("is_loaded", mod_api::__private::LoadAccess::WorldOnly);
        Self::native_is_loaded(id)
    }
    fn config_files(&self) -> &'static dyn ConfigFiles {
        mod_api::__private::check_access("config_files", mod_api::__private::LoadAccess::WorldOnly);
        Self::native_config_files()
    }
    fn recipes_of_type(&self, kind: &str) -> Arc<[Recipe]> {
        mod_api::__private::check_access(
            "recipes_of_type",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipes_of_type(kind)
    }
    fn recipe_ingredient(&self, json: &serde_json::Value) -> Option<Ingredient> {
        mod_api::__private::check_access(
            "recipe_ingredient",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_ingredient(json)
    }
    fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "recipe_result",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_result(json)
    }
    fn known_recipes(&mut self) -> Arc<[i32]> {
        mod_api::__private::check_access(
            "known_recipes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_known_recipes()
    }
    fn dimension(&self) -> &'static str {
        mod_api::__private::check_access("dimension", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dimension()
    }
    fn instance_dir(&self) -> PathBuf {
        mod_api::__private::check_access("instance_dir", mod_api::__private::LoadAccess::WorldOnly);
        self.native_instance_dir()
    }
}
impl BaseServerHost for Game<'_> {
    fn kill_mob(&mut self, dimension: &str, id: u32) -> bool {
        mod_api::__private::check_access("kill_mob", mod_api::__private::LoadAccess::WorldOnly);
        self.native_kill_mob(dimension, id)
    }
    fn summon(&mut self, dimension: &str, kind: &str, pos: [f64; 3], nbt: Compound) -> Option<u32> {
        mod_api::__private::check_access("summon", mod_api::__private::LoadAccess::WorldOnly);
        self.native_summon(dimension, kind, pos, nbt)
    }
    fn tick_stats(&self) -> (f32, f64) {
        mod_api::__private::check_access("tick_stats", mod_api::__private::LoadAccess::WorldOnly);
        self.native_tick_stats()
    }
    fn world_dir(&self) -> Option<PathBuf> {
        mod_api::__private::check_access("world_dir", mod_api::__private::LoadAccess::WorldOnly);
        self.native_world_dir()
    }
    fn edit_blocks(&mut self, dimension: &str, edit: &NativeBlockEdit) -> Result<u64, String> {
        mod_api::__private::check_access("edit_blocks", mod_api::__private::LoadAccess::WorldOnly);
        self.native_edit_blocks(dimension, edit)
    }
    fn surface_heights(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
    ) -> Result<Vec<Option<i32>>, String> {
        mod_api::__private::check_access(
            "surface_heights",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_surface_heights(dimension, columns, heightmap)
    }
    fn read_columns(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
        range: (u32, u32),
    ) -> Result<NativeColumnBlocks, String> {
        mod_api::__private::check_access("read_columns", mod_api::__private::LoadAccess::WorldOnly);
        self.native_read_columns(dimension, columns, heightmap, range)
    }
    fn set_experience_orb(
        &mut self,
        dimension: &str,
        id: u32,
        value: Option<u32>,
        count: Option<u32>,
        age: Option<u32>,
    ) -> bool {
        mod_api::__private::check_access(
            "set_experience_orb",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_experience_orb(dimension, id, value, count, age)
    }
    fn experience_orbs(
        &self,
        dimension: &str,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Vec<NativeExperienceOrb> {
        mod_api::__private::check_access(
            "experience_orbs",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_experience_orbs(dimension, min, max)
    }
    fn spawn_falling_block(
        &mut self,
        dimension: &str,
        state: BlockStateId,
        pos: [f64; 3],
        velocity: [f64; 3],
        falling: NativeFalling,
    ) -> bool {
        mod_api::__private::check_access(
            "spawn_falling_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_spawn_falling_block(dimension, state, pos, velocity, falling)
    }
    fn block_entity_sync(&mut self, pos: [i32; 3]) {
        mod_api::__private::check_access(
            "block_entity_sync",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_sync(pos)
    }
    fn set_block_property(&mut self, pos: [i32; 3], key: &str, value: &str) -> bool {
        mod_api::__private::check_access(
            "set_block_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_block_property(pos, key, value)
    }
    fn set_block_property_with(
        &mut self,
        pos: [i32; 3],
        key: &str,
        value: &str,
        flags: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "set_block_property_with",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_block_property_with(pos, key, value, flags)
    }
    fn net_send_to_player(
        &mut self,
        player: u128,
        channel: &str,
        data: &[u8],
    ) -> Result<(), String> {
        mod_api::__private::check_access(
            "net_send_to_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_net_send_to_player(player, channel, data)
    }
    fn dispense_default(
        &mut self,
        dimension: &str,
        pos: [i32; 3],
        facing: Direction,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "dispense_default",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dispense_default(dimension, pos, facing, stack)
    }
    fn open_menu(
        &mut self,
        uuid: u128,
        rows: u8,
        title: Text,
        items: Vec<Option<ItemStack>>,
    ) -> Option<i32> {
        mod_api::__private::check_access("open_menu", mod_api::__private::LoadAccess::WorldOnly);
        self.native_open_menu(uuid, rows, title, items)
    }
    fn close_menu(&mut self, uuid: u128) {
        mod_api::__private::check_access("close_menu", mod_api::__private::LoadAccess::WorldOnly);
        self.native_close_menu(uuid)
    }
}
impl BaseClientHost for Game<'_> {
    fn flame_particle(&mut self, at: [f64; 3], soul: bool) {
        mod_api::__private::check_access(
            "flame_particle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_flame_particle(at, soul)
    }
    fn refresh_biome_colors(&mut self) {
        mod_api::__private::check_access(
            "refresh_biome_colors",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_refresh_biome_colors()
    }
    fn mod_screen_open(&mut self, mod_id: &str) -> bool {
        mod_api::__private::check_access(
            "mod_screen_open",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mod_screen_open(mod_id)
    }
    fn mod_screen_close(&mut self, mod_id: &str) {
        mod_api::__private::check_access(
            "mod_screen_close",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mod_screen_close(mod_id)
    }
    fn mod_screen(&self) -> Option<&str> {
        mod_api::__private::check_access("mod_screen", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mod_screen()
    }
    fn clock(&self) -> Option<NativeClock> {
        mod_api::__private::check_access("clock", mod_api::__private::LoadAccess::ClientSetup);
        self.native_clock()
    }
    fn raycast(&self, distance: f32) -> Option<NativeHit> {
        mod_api::__private::check_access("raycast", mod_api::__private::LoadAccess::ClientSetup);
        self.native_raycast(distance)
    }
    fn view_hit(&self) -> Option<(f32, NativeHit)> {
        mod_api::__private::check_access("view_hit", mod_api::__private::LoadAccess::ClientSetup);
        self.native_view_hit()
    }
    fn claim_items(&mut self, mod_id: &str, ids: &[String]) {
        mod_api::__private::check_access("claim_items", mod_api::__private::LoadAccess::WorldOnly);
        self.native_claim_items(mod_id, ids)
    }
    fn poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        mod_api::__private::check_access("poll", mod_api::__private::LoadAccess::WorldOnly);
        self.native_poll(mod_id)
    }
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> bool {
        mod_api::__private::check_access(
            "play_local_sound",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_play_local_sound(sound, after)
    }
    fn particle_count(&self) -> u32 {
        mod_api::__private::check_access(
            "particle_count",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_particle_count()
    }
    fn clear_particles(&self) -> bool {
        mod_api::__private::check_access(
            "clear_particles",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_clear_particles()
    }
    fn screen_title(&self) -> Option<String> {
        mod_api::__private::check_access(
            "screen_title",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_title()
    }
    fn cursor(&self) -> [f32; 2] {
        mod_api::__private::check_access("cursor", mod_api::__private::LoadAccess::ClientSetup);
        self.native_cursor()
    }
    fn mod_key(&self, mod_id: &str, name: &str) -> Option<(String, ClientModifiers)> {
        mod_api::__private::check_access("mod_key", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mod_key(mod_id, name)
    }
    fn panel_items(&mut self, mod_id: &str, items: Vec<NativeStack>) {
        mod_api::__private::check_access("panel_items", mod_api::__private::LoadAccess::WorldOnly);
        self.native_panel_items(mod_id, items)
    }
    fn place_recipe(&mut self, book_id: i32, all: bool) -> Result<(), String> {
        mod_api::__private::check_access("place_recipe", mod_api::__private::LoadAccess::WorldOnly);
        self.native_place_recipe(book_id, all)
    }
    fn give(&mut self, item: &str, count: u32) -> Result<(), String> {
        mod_api::__private::check_access("give", mod_api::__private::LoadAccess::WorldOnly);
        self.native_give(item, count)
    }
    fn push_player(&mut self, kick: [f32; 3]) {
        mod_api::__private::check_access("push_player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_push_player(kick)
    }
    fn net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String> {
        mod_api::__private::check_access(
            "net_send_to_server",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_net_send_to_server(channel, data)
    }
    fn mod_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> bool {
        mod_api::__private::check_access("mod_image", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mod_image(width, height, rgba)
    }
}
impl BaseBackend for Game<'_> {
    fn player_address(&self, player: PlayerRef) -> Option<String> {
        mod_api::__private::check_access(
            "player_address",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_address(player)
    }
    fn player_name(&self, player: PlayerRef) -> Option<String> {
        mod_api::__private::check_access("player_name", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_name(player)
    }
}
impl BaseBackendMut for Game<'_> {
    fn player_tell(&mut self, player: PlayerRef, line: Text) -> bool {
        mod_api::__private::check_access("player_tell", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_tell(player, line)
    }
    fn player_action_bar(&mut self, player: PlayerRef, line: Text) -> bool {
        mod_api::__private::check_access(
            "player_action_bar",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_action_bar(player, line)
    }
    fn player_tab_list(&mut self, player: PlayerRef, header: &Text, footer: &Text) -> bool {
        mod_api::__private::check_access(
            "player_tab_list",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_tab_list(player, header, footer)
    }
    fn player_set_sleep_ignored(&mut self, player: PlayerRef, ignored: bool) -> bool {
        mod_api::__private::check_access(
            "player_set_sleep_ignored",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_sleep_ignored(player, ignored)
    }
    fn player_set_nickname(&mut self, player: PlayerRef, nickname: Option<&str>) -> bool {
        mod_api::__private::check_access(
            "player_set_nickname",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_nickname(player, nickname)
    }
    fn player_hide(&mut self, player: PlayerRef, level: Option<u8>) -> bool {
        mod_api::__private::check_access("player_hide", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_hide(player, level)
    }
}
