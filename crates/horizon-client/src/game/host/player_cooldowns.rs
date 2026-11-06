//! The game's side of the `player_cooldowns` area (`horizon-mod-api/src/host/player_cooldowns.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl PlayerCooldownsHost for Game<'_> {
    fn item_cooldown_group(&self, stack: &ItemStack) -> String {
        mod_api::__private::check_access(
            "item_cooldown_group",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_cooldown_group(stack)
    }
}
impl PlayerCooldownsBackend for Game<'_> {
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
}
impl PlayerCooldownsBackendMut for Game<'_> {
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
}
