//! The game's side of the `players` area (`horizon-mod-api/src/host/players.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use std::sync::Arc;
impl PlayersHost for Game<'_> {
    fn item_cooldown_group(&self, stack: &ItemStack) -> String {
        mod_api::__private::check_access(
            "item_cooldown_group",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_cooldown_group(stack)
    }
    fn player(&self, uuid: u128) -> Option<PlayerRef> {
        mod_api::__private::check_access("player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_ref(uuid)
    }
}
impl PlayersServerHost for Game<'_> {
    fn fake_player(&mut self, level: LevelRef, uuid: u128, name: &str) -> Option<PlayerRef> {
        mod_api::__private::check_access("fake_player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_fake_player(level, uuid, name)
    }
}
impl PlayersClientHost for Game<'_> {
    fn push_player(&mut self, kick: [f32; 3]) {
        mod_api::__private::check_access("push_player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_push_player(kick)
    }
}
impl PlayersBackend for Game<'_> {
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
    fn player_selected_slot(&self, player: PlayerRef) -> Option<usize> {
        mod_api::__private::check_access(
            "player_selected_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_selected_slot(player)
    }
    fn player_container_menu(&self, player: PlayerRef) -> Option<MenuRef> {
        mod_api::__private::check_access(
            "player_container_menu",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_ref(player)
    }
    fn player_item_cooldowns(&self, player: PlayerRef) -> Option<Vec<NativeItemCooldown>> {
        mod_api::__private::check_access(
            "player_item_cooldowns",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_item_cooldowns(player)
    }
    fn player_has_item_cooldown(&self, player: PlayerRef, group: &str) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_item_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_item_cooldown(player, group)
    }
    fn player_item_cooldown_percent(&self, player: PlayerRef, group: &str) -> Option<f32> {
        mod_api::__private::check_access(
            "player_item_cooldown_percent",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_item_cooldown_percent(player, group)
    }
    fn player_exp_cooldown(&self, player: PlayerRef) -> Option<u32> {
        mod_api::__private::check_access(
            "player_exp_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_exp_cooldown(player)
    }
    fn player_last_death_location(&self, player: PlayerRef) -> Option<GlobalPos> {
        mod_api::__private::check_access(
            "player_last_death_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_last_death_location(player)
    }
    fn player_needs_food(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_needs_food",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_needs_food(player)
    }
    fn player_has_enough_food(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_enough_food",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_enough_food(player)
    }
    fn player_can_eat(&self, player: PlayerRef, always: bool) -> Option<bool> {
        mod_api::__private::check_access(
            "player_can_eat",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_can_eat(player, always)
    }
    fn player_experience(&self, player: PlayerRef) -> Option<NativePlayerExperience> {
        mod_api::__private::check_access(
            "player_experience",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_experience(player)
    }
    fn player_xp_needed_for_next_level(&self, player: PlayerRef) -> Option<i32> {
        mod_api::__private::check_access(
            "player_xp_needed_for_next_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_xp_needed_for_next_level(player)
    }
    fn player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        mod_api::__private::check_access(
            "player_permission_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_permission_level(player)
    }
    fn player_has_infinite_materials(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_infinite_materials",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_infinite_materials(player)
    }
    fn player_can_use_game_master_blocks(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_can_use_game_master_blocks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_can_use_game_master_blocks(player)
    }
    fn player_previous_game_mode(&self, player: PlayerRef) -> Option<Option<GameMode>> {
        mod_api::__private::check_access(
            "player_previous_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_previous_game_mode(player)
    }
    fn player_is_secondary_use_active(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_secondary_use_active",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_secondary_use_active(player)
    }
    fn player_is_fake(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_fake",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_fake(player)
    }
    fn player_inventory(&self, player: PlayerRef) -> Option<Vec<ItemStack>> {
        mod_api::__private::check_access(
            "player_inventory",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_inventory(player)
    }
    fn player_game_mode(&self, player: PlayerRef) -> Option<GameMode> {
        mod_api::__private::check_access(
            "player_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_game_mode(player)
    }
    fn player_is_creative(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_creative",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_creative(player)
    }
    fn player_is_spectator(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_spectator",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_spectator(player)
    }
    fn player_may_build(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_may_build",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_may_build(player)
    }
    fn player_food(&self, player: PlayerRef) -> Option<NativePlayerFood> {
        mod_api::__private::check_access("player_food", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_food(player)
    }
    fn player_abilities(&self, player: PlayerRef) -> Option<NativePlayerAbilities> {
        mod_api::__private::check_access(
            "player_abilities",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_abilities(player)
    }
    fn player_stat(&self, player: PlayerRef, stat_type: NativeStatType, key: &str) -> Option<i32> {
        mod_api::__private::check_access("player_stat", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_stat(player, stat_type, key)
    }
    fn player_has_discovered_recipe(&self, player: PlayerRef, recipe: &str) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_discovered_recipe",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_discovered_recipe(player, recipe)
    }
    fn player_discovered_recipes(&self, player: PlayerRef) -> Option<Vec<String>> {
        mod_api::__private::check_access(
            "player_discovered_recipes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_discovered_recipes(player)
    }
}
impl PlayersClientBackend for Game<'_> {
    fn player_block_breaking(&self, player: PlayerRef) -> Option<BlockBreakingState> {
        mod_api::__private::check_access(
            "player_block_breaking",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_block_breaking(player)
    }
    fn player_ambient_mood(&self, player: PlayerRef) -> Option<f32> {
        mod_api::__private::check_access(
            "player_ambient_mood",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_ambient_mood(player)
    }
}
impl PlayersBackendMut for Game<'_> {
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
    fn player_open_menu(&mut self, player: PlayerRef, entity: BlockEntityRef) -> bool {
        mod_api::__private::check_access(
            "player_open_menu",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_open_block_menu(player, entity)
    }
    fn player_send_map(&mut self, player: PlayerRef, map_id: i32, colors: &[u8]) -> bool {
        mod_api::__private::check_access(
            "player_send_map",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_send_map(player, map_id, colors)
    }
    fn player_set_abilities(
        &mut self,
        player: PlayerRef,
        abilities: NativePlayerAbilities,
    ) -> bool {
        mod_api::__private::check_access(
            "player_set_abilities",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_abilities(player, abilities)
    }
    fn player_perform_action(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
    ) -> Result<ActionOutcome, ActionRefused> {
        mod_api::__private::check_access(
            "player_perform_action",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_perform_action(player, action)
    }
    fn player_perform_action_scoped(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
        setup: PlayerActionSetup,
    ) -> Result<ScopedActionResult, ActionRefused> {
        mod_api::__private::check_access(
            "player_perform_action_scoped",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_perform_action_scoped(player, action, setup)
    }
    fn player_set_item_cooldown(&mut self, player: PlayerRef, group: &str, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_item_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_item_cooldown(player, group, ticks)
    }
    fn player_remove_item_cooldown(&mut self, player: PlayerRef, group: &str) -> bool {
        mod_api::__private::check_access(
            "player_remove_item_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_remove_item_cooldown(player, group)
    }
    fn player_set_exp_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        mod_api::__private::check_access(
            "player_set_exp_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_exp_cooldown(player, ticks)
    }
    fn player_set_warden_warning_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        mod_api::__private::check_access(
            "player_set_warden_warning_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_warden_warning_cooldown(player, ticks)
    }
    fn player_trigger_criterion(
        &mut self,
        player: PlayerRef,
        trigger: &Identifier,
        test: Arc<dyn CriterionTest>,
    ) -> Result<(), TriggerCriterionError> {
        mod_api::__private::check_access(
            "player_trigger_criterion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_trigger_criterion(player, trigger, test)
    }
    fn player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<GlobalPos>,
    ) -> bool {
        mod_api::__private::check_access(
            "player_set_last_death_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_last_death_location(player, location)
    }
    fn player_feed(&mut self, player: PlayerRef, nutrition: i32, saturation_modifier: f32) -> bool {
        mod_api::__private::check_access("player_feed", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_feed(player, nutrition, saturation_modifier)
    }
    fn player_add_exhaustion(&mut self, player: PlayerRef, amount: f32) -> bool {
        mod_api::__private::check_access(
            "player_add_exhaustion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_add_exhaustion(player, amount)
    }
    fn player_set_food_level(&mut self, player: PlayerRef, level: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_food_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_food_level(player, level)
    }
    fn player_set_saturation(&mut self, player: PlayerRef, saturation: f32) -> bool {
        mod_api::__private::check_access(
            "player_set_saturation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_saturation(player, saturation)
    }
    fn player_set_exhaustion(&mut self, player: PlayerRef, exhaustion: f32) -> bool {
        mod_api::__private::check_access(
            "player_set_exhaustion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_exhaustion(player, exhaustion)
    }
    fn player_set_experience_points(&mut self, player: PlayerRef, points: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_experience_points",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_experience_points(player, points)
    }
    fn player_set_experience_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_experience_levels",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_experience_levels(player, levels)
    }
    fn player_set_game_mode(&mut self, player: PlayerRef, mode: GameMode) -> bool {
        mod_api::__private::check_access(
            "player_set_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_game_mode(player, mode)
    }
    fn player_remove_fake(&mut self, player: PlayerRef) -> bool {
        mod_api::__private::check_access(
            "player_remove_fake",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_remove_fake(player)
    }
    fn player_award_stat(
        &mut self,
        player: PlayerRef,
        stat_type: NativeStatType,
        key: &str,
        amount: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "player_award_stat",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_award_stat(player, stat_type, key, amount)
    }
    fn player_reset_stat(
        &mut self,
        player: PlayerRef,
        stat_type: NativeStatType,
        key: &str,
    ) -> bool {
        mod_api::__private::check_access(
            "player_reset_stat",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_reset_stat(player, stat_type, key)
    }
    fn player_award_recipes(&mut self, player: PlayerRef, recipes: &[&str]) -> Option<i32> {
        mod_api::__private::check_access(
            "player_award_recipes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_award_recipes(player, recipes)
    }
    fn player_reset_recipes(&mut self, player: PlayerRef, recipes: &[&str]) -> Option<i32> {
        mod_api::__private::check_access(
            "player_reset_recipes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_reset_recipes(player, recipes)
    }
    fn player_give_experience_points(&mut self, player: PlayerRef, points: u32) -> bool {
        mod_api::__private::check_access(
            "player_give_experience_points",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_give_experience_points(player, points)
    }
    fn player_give_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        mod_api::__private::check_access(
            "player_give_levels",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_give_levels(player, levels)
    }
    fn player_stop_sound(
        &mut self,
        player: PlayerRef,
        sound: Option<&str>,
        source: Option<SoundSource>,
    ) -> bool {
        mod_api::__private::check_access(
            "player_stop_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_stop_sound(player, sound, source)
    }
    fn player_play_sound(
        &mut self,
        player: PlayerRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "player_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_play_sound(player, sound, source, volume, pitch)
    }
}
