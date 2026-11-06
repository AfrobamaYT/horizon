//! The game's side of the `raid_refs` area (`horizon-mod-api/src/host/raid_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl RaidRefsBackend for Game<'_> {
    fn level_raid(&self, level: LevelRef, id: i32) -> Option<RaidRef> {
        mod_api::__private::check_access("level_raid", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_raid(level, id)
    }
    fn level_raid_at(&self, level: LevelRef, pos: [i32; 3]) -> Option<RaidRef> {
        mod_api::__private::check_access(
            "level_raid_at",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_raid_at(level, pos)
    }
    fn level_raids(&self, level: LevelRef) -> Option<Vec<RaidRef>> {
        mod_api::__private::check_access("level_raids", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_raids(level)
    }
    fn mob_raid(&self, mob: MobRef) -> Option<RaidRef> {
        mod_api::__private::check_access("mob_raid", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_raid(mob)
    }
    fn raid_center(&self, raid: RaidRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access("raid_center", mod_api::__private::LoadAccess::WorldOnly);
        self.native_raid_center(raid)
    }
    fn raid_omen_level(&self, raid: RaidRef) -> Option<i32> {
        mod_api::__private::check_access(
            "raid_omen_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_omen_level(raid)
    }
    fn raid_groups_spawned(&self, raid: RaidRef) -> Option<i32> {
        mod_api::__private::check_access(
            "raid_groups_spawned",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_groups_spawned(raid)
    }
    fn raid_num_groups(&self, raid: RaidRef) -> Option<i32> {
        mod_api::__private::check_access(
            "raid_num_groups",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_num_groups(raid)
    }
    fn raid_has_bonus_wave(&self, raid: RaidRef) -> Option<bool> {
        mod_api::__private::check_access(
            "raid_has_bonus_wave",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_has_bonus_wave(raid)
    }
    fn raid_is_started(&self, raid: RaidRef) -> bool {
        mod_api::__private::check_access(
            "raid_is_started",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_is_started(raid)
    }
    fn raid_is_active(&self, raid: RaidRef) -> bool {
        mod_api::__private::check_access(
            "raid_is_active",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_is_active(raid)
    }
    fn raid_status(&self, raid: RaidRef) -> Option<NativeRaidStatus> {
        mod_api::__private::check_access("raid_status", mod_api::__private::LoadAccess::WorldOnly);
        self.native_raid_status(raid)
    }
    fn raid_total_health(&self, raid: RaidRef) -> Option<f32> {
        mod_api::__private::check_access(
            "raid_total_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_total_health(raid)
    }
    fn raid_living_health(&self, raid: RaidRef) -> Option<f32> {
        mod_api::__private::check_access(
            "raid_living_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_living_health(raid)
    }
    fn raid_bar_progress(&self, raid: RaidRef) -> Option<f32> {
        mod_api::__private::check_access(
            "raid_bar_progress",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_bar_progress(raid)
    }
    fn raid_raiders(&self, raid: RaidRef) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access("raid_raiders", mod_api::__private::LoadAccess::WorldOnly);
        self.native_raid_raiders(raid)
    }
    fn raid_wave_raiders(&self, raid: RaidRef, wave: i32) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access(
            "raid_wave_raiders",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_wave_raiders(raid, wave)
    }
    fn raid_heroes(&self, raid: RaidRef) -> Option<Vec<u128>> {
        mod_api::__private::check_access("raid_heroes", mod_api::__private::LoadAccess::WorldOnly);
        self.native_raid_heroes(raid)
    }
}
impl RaidRefsBackendMut for Game<'_> {
    fn raid_set_omen_level(&mut self, raid: RaidRef, level: i32) -> bool {
        mod_api::__private::check_access(
            "raid_set_omen_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_raid_set_omen_level(raid, level)
    }
    fn raid_stop(&mut self, raid: RaidRef) -> bool {
        mod_api::__private::check_access("raid_stop", mod_api::__private::LoadAccess::WorldOnly);
        self.native_raid_stop(raid)
    }
}
