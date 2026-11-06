//! The `player_cooldowns` area: host methods the game implements (`game/host/player_cooldowns.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `player_cooldowns` area, implemented by the game and reached through [`Host`].
pub trait PlayerCooldownsHost {
    /// Side: both.
    /// `ItemCooldowns.getCooldownGroup(stack)`: the group `stack`'s
    /// cooldown is filed under, the group its `use_cooldown` component names
    /// (its own, else its item's default), else its item id.
    fn item_cooldown_group(&self, stack: &ItemStack) -> String {
        crate::phase::unprovided(
            "Host::item_cooldown_group",
            "item_cooldown_group",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `player_cooldowns` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerCooldownsBackend {
    /// Side: server.
    /// Every group the player waits on and its ticks left, ordered by
    /// group.
    fn player_item_cooldowns(&self, player: PlayerRef) -> Option<Vec<NativeItemCooldown>> {
        crate::phase::unprovided(
            "Backend::player_item_cooldowns",
            "player_item_cooldowns",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.isOnCooldown`: whether `group` still has time to run.
    fn player_has_item_cooldown(&self, player: PlayerRef, group: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_item_cooldown",
            "player_has_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.getCooldownPercent(group, 0)`: what is left of the
    /// group's cooldown, from 1 (just started) to 0 (none).
    fn player_item_cooldown_percent(&self, player: PlayerRef, group: &str) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::player_item_cooldown_percent",
            "player_item_cooldown_percent",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.takeXpDelay`: the ticks before the player can take the next
    /// experience orb.
    fn player_exp_cooldown(&self, player: PlayerRef) -> Option<u32> {
        crate::phase::unprovided(
            "Backend::player_exp_cooldown",
            "player_exp_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `player_cooldowns` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerCooldownsBackendMut {
    /// Side: server.
    /// `ItemCooldowns.addCooldown(group, ticks)`: starts (or restarts) the
    /// group's cooldown, and the player's client draws it on every item of the
    /// group (`ServerItemCooldowns.onCooldownStarted`).
    fn player_set_item_cooldown(&mut self, player: PlayerRef, group: &str, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_item_cooldown",
            "player_set_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.removeCooldown(group)`: ends the group's cooldown now,
    /// on the client too (`ServerItemCooldowns.onCooldownEnded`).
    fn player_remove_item_cooldown(&mut self, player: PlayerRef, group: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_remove_item_cooldown",
            "player_remove_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets `Player.takeXpDelay`: no orb is taken for `ticks` ticks. The
    /// change is not reported to `player_exp_cooldown_change`.
    fn player_set_exp_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_exp_cooldown",
            "player_set_exp_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the player's `WardenSpawnTracker.cooldownTicks`: sculk shriekers
    /// they set off don't warn for `ticks` ticks. The level's players share
    /// one tracker here, the integrated server owner's (the shriekers read it
    /// for whoever sets them off), so this is every player's.
    /// False, changing nothing, for anyone but the tracker's owner until each player has their own.
    fn player_set_warden_warning_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_warden_warning_cooldown",
            "player_set_warden_warning_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
