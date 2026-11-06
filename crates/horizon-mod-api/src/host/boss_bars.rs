//! The `boss_bars` area: host methods the game implements (`game/host/boss_bars.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `boss_bars` area, implemented by the game and reached through [`ServerHost`].
pub trait BossBarsServerHost {
    /// Side: server.
    /// An existing custom boss bar by id (`minecraft:` when no namespace), or
    /// None. Saved bars are there from the world's start, as vanilla's
    /// `CustomBossEvents`. Boss bars of mobs and raids are not custom boss
    /// events.
    fn boss_bar(&self, id: &str) -> Option<BossBarRef> {
        crate::phase::unprovided(
            "ServerHost::boss_bar",
            "boss_bar",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Every custom boss bar, in the order `/bossbar list` shows them.
    fn boss_bars(&self) -> Vec<BossBarRef> {
        crate::phase::unprovided(
            "ServerHost::boss_bars",
            "boss_bars",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `/bossbar add`: a new bar, visible, white, `progress`, at 0 of 100, with
    /// nobody added. None if the id is taken or invalid, or outside a server.
    fn create_boss_bar(&mut self, id: &str, name: Text) -> Option<BossBarRef> {
        crate::phase::unprovided(
            "ServerHost::create_boss_bar",
            "create_boss_bar",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `boss_bars` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BossBarsBackend {
    /// Side: server.
    /// The bar's id (`CustomBossEvent.customId`), or None for a stale handle.
    fn boss_bar_id(&self, bar: BossBarRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::boss_bar_id",
            "boss_bar_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.getName`.
    fn boss_bar_name(&self, bar: BossBarRef) -> Option<Text> {
        crate::phase::unprovided(
            "Backend::boss_bar_name",
            "boss_bar_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.getColor`.
    fn boss_bar_color(&self, bar: BossBarRef) -> Option<BossBarColor> {
        crate::phase::unprovided(
            "Backend::boss_bar_color",
            "boss_bar_color",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.getOverlay`.
    fn boss_bar_overlay(&self, bar: BossBarRef) -> Option<BossBarOverlay> {
        crate::phase::unprovided(
            "Backend::boss_bar_overlay",
            "boss_bar_overlay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.shouldDarkenScreen`.
    fn boss_bar_darken_screen(&self, bar: BossBarRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::boss_bar_darken_screen",
            "boss_bar_darken_screen",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.shouldPlayBossMusic`.
    fn boss_bar_play_boss_music(&self, bar: BossBarRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::boss_bar_play_boss_music",
            "boss_bar_play_boss_music",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BossEvent.shouldCreateWorldFog`.
    fn boss_bar_create_world_fog(&self, bar: BossBarRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::boss_bar_create_world_fog",
            "boss_bar_create_world_fog",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.value`.
    fn boss_bar_value(&self, bar: BossBarRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::boss_bar_value",
            "boss_bar_value",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.max`.
    fn boss_bar_max(&self, bar: BossBarRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::boss_bar_max",
            "boss_bar_max",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.isVisible`.
    fn boss_bar_visible(&self, bar: BossBarRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::boss_bar_visible",
            "boss_bar_visible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.getPlayers`: the bar's players who are online.
    fn boss_bar_players(&self, bar: BossBarRef) -> Option<Vec<PlayerRef>> {
        crate::phase::unprovided(
            "Backend::boss_bar_players",
            "boss_bar_players",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' writes of the `boss_bars` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BossBarsBackendMut {
    /// Side: server.
    /// `ServerBossEvent.setName`: viewers are sent the new name.
    fn boss_bar_set_name(&mut self, bar: BossBarRef, name: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_name",
            "boss_bar_set_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setColor`.
    fn boss_bar_set_color(&mut self, bar: BossBarRef, color: BossBarColor) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_color",
            "boss_bar_set_color",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setOverlay`.
    fn boss_bar_set_overlay(&mut self, bar: BossBarRef, overlay: BossBarOverlay) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_overlay",
            "boss_bar_set_overlay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setDarkenScreen`.
    fn boss_bar_set_darken_screen(&mut self, bar: BossBarRef, darken: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_darken_screen",
            "boss_bar_set_darken_screen",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setPlayBossMusic`.
    fn boss_bar_set_play_boss_music(&mut self, bar: BossBarRef, music: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_play_boss_music",
            "boss_bar_set_play_boss_music",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setCreateWorldFog`.
    fn boss_bar_set_create_world_fog(&mut self, bar: BossBarRef, fog: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_create_world_fog",
            "boss_bar_set_create_world_fog",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.setValue`: stored as given, as vanilla does; only the
    /// progress shown is kept between empty and full.
    fn boss_bar_set_value(&mut self, bar: BossBarRef, value: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_value",
            "boss_bar_set_value",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.setMax`: stored as given, and the value is kept. Only
    /// `/bossbar set max` limits it to 1 or more.
    fn boss_bar_set_max(&mut self, bar: BossBarRef, max: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_max",
            "boss_bar_set_max",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerBossEvent.setVisible`: a hidden bar is taken off its players'
    /// screens and keeps them.
    fn boss_bar_set_visible(&mut self, bar: BossBarRef, visible: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_set_visible",
            "boss_bar_set_visible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.addPlayer`. False for a stale bar or an offline or
    /// stale player. The bar is kept for the player across sessions.
    fn boss_bar_add_player(&mut self, bar: BossBarRef, player: PlayerRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_add_player",
            "boss_bar_add_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `CustomBossEvent.removePlayer`. False for a stale bar or an offline or
    /// stale player.
    fn boss_bar_remove_player(&mut self, bar: BossBarRef, player: PlayerRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_remove_player",
            "boss_bar_remove_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `/bossbar remove` (`CustomBossEvents.remove`): the bar's viewers lose it
    /// and its handle goes stale.
    fn boss_bar_remove(&mut self, bar: BossBarRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::boss_bar_remove",
            "boss_bar_remove",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
