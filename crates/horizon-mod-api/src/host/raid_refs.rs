//! The `raid_refs` area: host methods the game implements (`game/host/raid_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `raid_refs` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait RaidRefsBackend {
    /// Side: server.
    /// `Raids.get(id)`: the raid saved under `id` in the requested level, if
    /// `Raids` still holds it (a stopped raid goes at its next tick).
    fn level_raid(&self, level: LevelRef, id: i32) -> Option<RaidRef> {
        crate::phase::unprovided(
            "Backend::level_raid",
            "level_raid",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerLevel.getRaidAt(pos)`: the closest active raid whose centre is
    /// less than 96 blocks away (`Raids.getNearbyRaid`, 9216 squared).
    fn level_raid_at(&self, level: LevelRef, pos: [i32; 3]) -> Option<RaidRef> {
        crate::phase::unprovided(
            "Backend::level_raid_at",
            "level_raid_at",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Every raid `Raids` holds, in its iteration order, stopped ones
    /// included until their next tick. None for a closed or stale level.
    fn level_raids(&self, level: LevelRef) -> Option<Vec<RaidRef>> {
        crate::phase::unprovided(
            "Backend::level_raids",
            "level_raids",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raider.getCurrentRaid`: the raid a raider belongs to, even a stopped
    /// one. None for a mob that is no raider or in no raid.
    fn mob_raid(&self, mob: MobRef) -> Option<RaidRef> {
        crate::phase::unprovided(
            "Backend::mob_raid",
            "mob_raid",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getCenter`.
    fn raid_center(&self, raid: RaidRef) -> Option<[i32; 3]> {
        crate::phase::unprovided(
            "Backend::raid_center",
            "raid_center",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getRaidOmenLevel`: 0 to 5 in play, any value a mod set.
    fn raid_omen_level(&self, raid: RaidRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::raid_omen_level",
            "raid_omen_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getGroupsSpawned`: the waves spawned so far.
    fn raid_groups_spawned(&self, raid: RaidRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::raid_groups_spawned",
            "raid_groups_spawned",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.numGroups`, set from `getNumGroups(difficulty)` when the raid
    /// began: 0, 3, 5 or 7. A raid of omen level 2 or more adds a bonus wave
    /// (`hasBonusWave`).
    fn raid_num_groups(&self, raid: RaidRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::raid_num_groups",
            "raid_num_groups",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the omen level is above 1, which adds a wave after the last
    /// (`Raid.hasBonusWave`).
    fn raid_has_bonus_wave(&self, raid: RaidRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::raid_has_bonus_wave",
            "raid_has_bonus_wave",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.isStarted`.
    fn raid_is_started(&self, raid: RaidRef) -> bool {
        crate::phase::unprovided(
            "Backend::raid_is_started",
            "raid_is_started",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.isActive`: false once stopped.
    fn raid_is_active(&self, raid: RaidRef) -> bool {
        crate::phase::unprovided(
            "Backend::raid_is_active",
            "raid_is_active",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.RaidStatus`.
    fn raid_status(&self, raid: RaidRef) -> Option<NativeRaidStatus> {
        crate::phase::unprovided(
            "Backend::raid_status",
            "raid_status",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getTotalHealth`: the health of every raider that joined, which
    /// the boss bar divides by.
    fn raid_total_health(&self, raid: RaidRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::raid_total_health",
            "raid_total_health",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getHealthOfLivingRaiders`.
    fn raid_living_health(&self, raid: RaidRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::raid_living_health",
            "raid_living_health",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The boss bar's progress, 0 to 1 (`Raid.raidEvent.getProgress`).
    fn raid_bar_progress(&self, raid: RaidRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::raid_bar_progress",
            "raid_bar_progress",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.getAllRaiders`: the living raiders that joined, in mob-list
    /// order, as entity handles.
    fn raid_raiders(&self, raid: RaidRef) -> Option<Vec<EntityRef>> {
        crate::phase::unprovided(
            "Backend::raid_raiders",
            "raid_raiders",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The living raiders of one wave (`groupRaiderMap.get(wave)`).
    fn raid_wave_raiders(&self, raid: RaidRef, wave: i32) -> Option<Vec<EntityRef>> {
        crate::phase::unprovided(
            "Backend::raid_wave_raiders",
            "raid_wave_raiders",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.heroesOfTheVillage`: the UUIDs of the players who got Hero of
    /// the Village, online or not, in the order vanilla's set iterates them.
    fn raid_heroes(&self, raid: RaidRef) -> Option<Vec<u128>> {
        crate::phase::unprovided(
            "Backend::raid_heroes",
            "raid_heroes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `raid_refs` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait RaidRefsBackendMut {
    /// Side: server.
    /// `Raid.setRaidOmenLevel`: any value, as vanilla assigns it (only
    /// `absorbRaidOmen` clamps, to 0 to 5). The raid is saved with it.
    fn raid_set_omen_level(&mut self, raid: RaidRef, level: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::raid_set_omen_level",
            "raid_set_omen_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Raid.stop`: the raid ends as `stopped` and its boss bar leaves the
    /// players; the `raid_state_change` hook fires. False if the raid is
    /// already stopped.
    fn raid_stop(&mut self, raid: RaidRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::raid_stop",
            "raid_stop",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
