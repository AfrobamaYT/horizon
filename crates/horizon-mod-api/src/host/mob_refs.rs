//! The `mob_refs` area: host methods the game implements (`game/host/mob_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `mob_refs` area, implemented by the game and reached through [`Host`].
pub trait MobRefsHost {
    /// Side: both.
    /// The entity as a vanilla Mob: a mob or the ender dragon. Players, armor
    /// stands, stale handles and clients are rejected. Mobs taken out for
    /// ticking are unavailable (a #29 follow-up).
    fn mob(&self, entity: EntityRef) -> Option<MobRef> {
        crate::phase::unprovided("Host::mob", "mob", crate::phase::LoadAccess::WorldOnly)
    }
}
/// The handles' reads of the `mob_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MobRefsBackend {
    /// Side: both.
    /// The existing portal decision for an available mob in the current level.
    fn mob_can_use_portal(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_can_use_portal",
            "mob_can_use_portal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The live leash holder, refusing unresolved saved and unavailable holders.
    fn mob_leash_holder(&self, mob: MobRef) -> Option<MobLeashHolder> {
        crate::phase::unprovided(
            "Backend::mob_leash_holder",
            "mob_leash_holder",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.getTarget`. Brain-backed mobs read their attack memory; piglins
    /// and hoglins use Horizon's `ai.target` stand-in. Invalid targets, an
    /// unavailable mob or player binding, or a target without a living handle
    /// read as None. The dragon reads what a mod set, while it stays a valid
    /// target (`EnderDragon.canAttack`); its flight never sets one.
    fn mob_target(&self, mob: MobRef) -> Option<LivingRef> {
        crate::phase::unprovided(
            "Backend::mob_target",
            "mob_target",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.isAggressive`, the synced mob flag (the dragon's reaches its
    /// clients as `DATA_MOB_FLAGS_ID`).
    fn mob_is_aggressive(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_is_aggressive",
            "mob_is_aggressive",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.isPersistenceRequired`. The dragon never despawns
    /// (`EnderDragon.checkDespawn` is empty), but keeps and saves the flag
    /// (`PersistenceRequired`).
    fn mob_persistence_required(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_persistence_required",
            "mob_persistence_required",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.requiresCustomPersistence` and its species overrides: passengers,
    /// leashed mobs and species-specific reasons to stay, such as an enderman
    /// carrying a block. Independent of `PersistenceRequired` and `NoAI`.
    /// None for an unavailable mob or the separately modeled dragon.
    fn mob_requires_custom_persistence(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_requires_custom_persistence",
            "mob_requires_custom_persistence",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.isNoAi`: the mob stands where it is and its goals do not run,
    /// while what NoAI keeps running in vanilla (the daylight burn of the
    /// undead, a horse's tail) still does. Saved with it as `NoAI`. None
    /// for anything but a mob.
    fn mob_no_ai(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_no_ai",
            "mob_no_ai",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.getRestrictCenter`: the restriction's block position, kept
    /// after `clear_restriction` and after a load (`[0, 0, 0]` for a mob
    /// that never had one, vanilla's `ZERO`). None for a stale or
    /// unavailable mob, a client, and the dragon.
    fn mob_restrict_center(&self, mob: MobRef) -> Option<[i32; 3]> {
        crate::phase::unprovided(
            "Backend::mob_restrict_center",
            "mob_restrict_center",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.getRestrictRadius`: `-1` while the mob has no restriction.
    /// None for a stale or unavailable mob, a client, and the dragon.
    fn mob_restrict_radius(&self, mob: MobRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::mob_restrict_radius",
            "mob_restrict_radius",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.hasRestriction`. None for a stale or unavailable mob, a client,
    /// and the dragon.
    fn mob_has_restriction(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_has_restriction",
            "mob_has_restriction",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.isWithinRestriction(pos)`: whether the block position is inside
    /// the home area; a mob without a restriction is within everywhere, as
    /// in vanilla. None for a stale or unavailable mob, a client, and the
    /// dragon.
    fn mob_is_within_restriction(&self, mob: MobRef, at: [i32; 3]) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_is_within_restriction",
            "mob_is_within_restriction",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.isLeftHanded`: whether the mob holds things in its left hand
    /// (the mob flags in its entity data). None for anything but a mob.
    fn mob_left_handed(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_left_handed",
            "mob_left_handed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.canPickUpLoot`, with the overrides of an allay (holding
    /// something) and a sulfur cube (with no block). None for anything but
    /// a mob.
    fn mob_can_pick_up_loot(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_can_pick_up_loot",
            "mob_can_pick_up_loot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.getNoActionTime`: the ticks since the mob last had a
    /// reason to stay (a player near, a hit). Despawning and a raid's
    /// recruiting read it. None for a stale or unavailable mob, the dragon,
    /// or on a client.
    fn mob_no_action_time(&self, mob: MobRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::mob_no_action_time",
            "mob_no_action_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `mob_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MobRefsBackendMut {
    /// Side: server.
    /// `Mob.setTarget`; None clears it. Only mobs and players whose binding
    /// can be stored in the current level are supported; armor stands are
    /// refused. The dragon stores a mob or a player, which its flight ignores
    /// and its save does not keep (vanilla keeps no target either). Brain-backed mobs refuse writes: vanilla's setter
    /// does not update their brain memory (#934). A fox, an enderman or a
    /// zombified piglin stores the target with its own `setTarget` side
    /// effects (`Mob::set_target`).
    /// Invalid targets are refused (false, mob unchanged); vanilla instead
    /// stores `asValidTarget(target)`, clearing the target when it is invalid.
    /// False also means a stale handle, client, or an unavailable mob/player.
    fn mob_set_target(&mut self, mob: MobRef, target: Option<LivingRef>) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_target",
            "mob_set_target",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setAggressive`. False for an unavailable mob.
    fn mob_set_aggressive(&mut self, mob: MobRef, value: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_aggressive",
            "mob_set_aggressive",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setPersistenceRequired`, which sets the flag and cannot clear it.
    /// False for an unavailable mob.
    fn mob_set_persistence_required(&mut self, mob: MobRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_persistence_required",
            "mob_set_persistence_required",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setNoAi`: whether the mob's own AI runs. With no AI the mob
    /// stands (what NoAI keeps in vanilla still runs), and the flag is
    /// saved with it as `NoAI`. False for anything but a mob.
    fn mob_set_no_ai(&mut self, mob: MobRef, no_ai: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_no_ai",
            "mob_set_no_ai",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setBaby` through the species' setter: ageable mobs use their
    /// baby start age (`-48000` for sniffers, otherwise `-24000`) or `0`,
    /// including age-boundary attribute and AI setup changes. Non-ageable
    /// babies, such as zombies, keep their non-growing baby state instead.
    /// Leaves `NoAI` and `PersistenceRequired` unchanged. False, with no
    /// changes, for an unavailable mob or a species with no baby form.
    fn mob_set_baby(&mut self, mob: MobRef, baby: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_baby",
            "mob_set_baby",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LookControl.setLookAt`, by the mob's `lookAt` path: the wanted
    /// position its look control rotates the head toward over the next
    /// ticks. As `move_to`'s path, the mob's own AI — a goal's look, or a
    /// brain species' `LookAtTargetSink`, which writes through this same
    /// control — may issue its own and replace this one; no species refuses.
    /// A creaking being looked at, an angry bee and an axolotl playing dead
    /// hold their look control, as in vanilla: the wanted position waits
    /// until it runs again. False for non-finite coordinates, a stale or
    /// unavailable mob, a client, and the dragon.
    fn mob_look_at(&mut self, mob: MobRef, at: [f64; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_look_at",
            "mob_look_at",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `JumpControl.jump`: the mob jumps on its next own tick by its own
    /// jump path — no velocity is written, and the control consumes the
    /// wish on that tick (a rabbit's jump control reads it too). The mob's
    /// own wishes (a slime's hop, a swimming brain's) are the same flag; no
    /// species refuses. A creaking being looked at holds its jump control,
    /// as in vanilla: the wish waits until it runs again. False for a stale
    /// or unavailable mob, a client, and the dragon.
    fn mob_jump(&mut self, mob: MobRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_jump",
            "mob_jump",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.restrictTo(pos, radius)`: the home area the mob keeps to — its
    /// own stroll and wander goals pick their positions inside it, as a
    /// lead's and a guardian's home do. Radius `-1` clears the restriction
    /// while keeping `pos` as the center, `clear_restriction`'s shape. The
    /// area is saved with the mob (`home_pos`, `home_radius`). False for a
    /// stale or unavailable mob, a client, and the dragon.
    fn mob_restrict_to(&mut self, mob: MobRef, at: [i32; 3], radius: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_restrict_to",
            "mob_restrict_to",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.clearRestriction`: the restriction goes, the center stays (its
    /// own goals read no restriction, as vanilla's `-1` radius). False for
    /// a stale or unavailable mob, a client, and the dragon.
    fn mob_clear_restriction(&mut self, mob: MobRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_clear_restriction",
            "mob_clear_restriction",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setLeftHanded`: sets the mob flag the players around get with
    /// the mob's next entity data and see in its swing and held items.
    /// Saved with the mob as `LeftHanded`. False for anything but a mob.
    fn mob_set_left_handed(&mut self, mob: MobRef, left_handed: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_left_handed",
            "mob_set_left_handed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setCanPickUpLoot`: whether the mob picks up the items it sees
    /// (saved with it as `CanPickUpLoot`; clients are not told, as in
    /// vanilla). An allay and a sulfur cube decide for themselves. False
    /// for anything but a mob.
    fn mob_set_can_pick_up_loot(&mut self, mob: MobRef, can_pick_up: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_can_pick_up_loot",
            "mob_set_can_pick_up_loot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.setNoActionTime`: the count the mob's own tick carries
    /// on from (a far-away mob's despawn chance starts past 600). Not saved,
    /// as in vanilla. False for a stale or unavailable mob, the dragon, or
    /// on a client.
    fn mob_set_no_action_time(&mut self, mob: MobRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_no_action_time",
            "mob_set_no_action_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
