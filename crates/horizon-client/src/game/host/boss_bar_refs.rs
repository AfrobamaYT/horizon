//! The game's side of the `boss_bar_refs` area (`horizon-mod-api/src/host/boss_bar_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl BossBarRefsServerHost for Game<'_> {
    fn boss_bar(&self, id: &str) -> Option<BossBarRef> {
        mod_api::__private::check_access(
            "boss_bar",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar(id)
    }
    fn boss_bars(&self) -> Vec<BossBarRef> {
        mod_api::__private::check_access(
            "boss_bars",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bars()
    }
    fn create_boss_bar(&mut self, id: &str, name: Text) -> Option<BossBarRef> {
        mod_api::__private::check_access(
            "create_boss_bar",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_create_boss_bar(id, name)
    }
}
impl BossBarRefsBackend for Game<'_> {
    fn boss_bar_id(&self, bar: BossBarRef) -> Option<String> {
        mod_api::__private::check_access(
            "boss_bar_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_id(bar)
    }
    fn boss_bar_name(&self, bar: BossBarRef) -> Option<Text> {
        mod_api::__private::check_access(
            "boss_bar_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_name(bar)
    }
    fn boss_bar_color(&self, bar: BossBarRef) -> Option<BossBarColor> {
        mod_api::__private::check_access(
            "boss_bar_color",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_color(bar)
    }
    fn boss_bar_overlay(&self, bar: BossBarRef) -> Option<BossBarOverlay> {
        mod_api::__private::check_access(
            "boss_bar_overlay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_overlay(bar)
    }
    fn boss_bar_darken_screen(&self, bar: BossBarRef) -> Option<bool> {
        mod_api::__private::check_access(
            "boss_bar_darken_screen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_darken_screen(bar)
    }
    fn boss_bar_play_boss_music(&self, bar: BossBarRef) -> Option<bool> {
        mod_api::__private::check_access(
            "boss_bar_play_boss_music",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_play_boss_music(bar)
    }
    fn boss_bar_create_world_fog(&self, bar: BossBarRef) -> Option<bool> {
        mod_api::__private::check_access(
            "boss_bar_create_world_fog",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_create_world_fog(bar)
    }
    fn boss_bar_value(&self, bar: BossBarRef) -> Option<i32> {
        mod_api::__private::check_access(
            "boss_bar_value",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_value(bar)
    }
    fn boss_bar_max(&self, bar: BossBarRef) -> Option<i32> {
        mod_api::__private::check_access(
            "boss_bar_max",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_max(bar)
    }
    fn boss_bar_visible(&self, bar: BossBarRef) -> Option<bool> {
        mod_api::__private::check_access(
            "boss_bar_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_visible(bar)
    }
    fn boss_bar_players(&self, bar: BossBarRef) -> Option<Vec<PlayerRef>> {
        mod_api::__private::check_access(
            "boss_bar_players",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_players(bar)
    }
}
impl BossBarRefsBackendMut for Game<'_> {
    fn boss_bar_set_name(&mut self, bar: BossBarRef, name: Text) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_name(bar, name)
    }
    fn boss_bar_set_color(&mut self, bar: BossBarRef, color: BossBarColor) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_color",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_color(bar, color)
    }
    fn boss_bar_set_overlay(
        &mut self,
        bar: BossBarRef,
        overlay: BossBarOverlay,
    ) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_overlay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_overlay(bar, overlay)
    }
    fn boss_bar_set_darken_screen(&mut self, bar: BossBarRef, darken: bool) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_darken_screen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_darken_screen(bar, darken)
    }
    fn boss_bar_set_play_boss_music(&mut self, bar: BossBarRef, music: bool) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_play_boss_music",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_play_boss_music(bar, music)
    }
    fn boss_bar_set_create_world_fog(&mut self, bar: BossBarRef, fog: bool) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_create_world_fog",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_create_world_fog(bar, fog)
    }
    fn boss_bar_set_value(&mut self, bar: BossBarRef, value: i32) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_value",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_value(bar, value)
    }
    fn boss_bar_set_max(&mut self, bar: BossBarRef, max: i32) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_max",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_max(bar, max)
    }
    fn boss_bar_set_visible(&mut self, bar: BossBarRef, visible: bool) -> bool {
        mod_api::__private::check_access(
            "boss_bar_set_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_set_visible(bar, visible)
    }
    fn boss_bar_add_player(&mut self, bar: BossBarRef, player: PlayerRef) -> bool {
        mod_api::__private::check_access(
            "boss_bar_add_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_add_player(bar, player)
    }
    fn boss_bar_remove_player(&mut self, bar: BossBarRef, player: PlayerRef) -> bool {
        mod_api::__private::check_access(
            "boss_bar_remove_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_remove_player(bar, player)
    }
    fn boss_bar_remove(&mut self, bar: BossBarRef) -> bool {
        mod_api::__private::check_access(
            "boss_bar_remove",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_boss_bar_remove(bar)
    }
}
