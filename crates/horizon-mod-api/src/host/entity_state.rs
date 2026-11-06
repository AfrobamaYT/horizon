//! The `entity_state` area: host methods the game implements (`game/host/entity_state.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_state` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityStateBackend {
    /// Side: both.
    /// `isSecondaryUseActive`: the local client's live semantic sneak state (not the
    /// physical key binding or last tick's crouching pose); the server's shift-key flag.
    /// False for non-player entities, None for a stale or unreachable handle.
    fn entity_is_shift_key_down(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_shift_key_down",
            "entity_is_shift_key_down",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity crouches (`Entity.isCrouching`: the crouching
    /// pose).
    fn entity_is_crouching(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_crouching",
            "entity_is_crouching",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity sprints (`Entity.isSprinting`): a player, or a
    /// cat or ocelot dashing after its target (their shared sprinting flag);
    /// false for the other kinds.
    fn entity_is_sprinting(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_sprinting",
            "entity_is_sprinting",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity touched water at its last fluid update
    /// (`Entity.isInWater`): players and mobs.
    fn entity_is_in_water(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_in_water",
            "entity_is_in_water",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity is in lava (`Entity.isInLava`): players and mobs.
    fn entity_is_in_lava(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_in_lava",
            "entity_is_in_lava",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity's type never burns (`EntityType.fireImmune`):
    /// players and mobs.
    fn entity_fire_immune(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_fire_immune",
            "entity_fire_immune",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The ticks the entity burns for (`Entity.getRemainingFireTicks`):
    /// players, mobs, armor stands and items.
    fn entity_fire_ticks(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_fire_ticks",
            "entity_fire_ticks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Entity.getPortalCooldown`: a player's own portal state, the falling
    /// block's, a pearl's End gateway state, or the level's for the other
    /// kinds its portal walk carries (zero when it keeps no state for one).
    fn entity_portal_cooldown(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_portal_cooldown",
            "entity_portal_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entity's air (`Entity.getAirSupply`): players and mobs.
    fn entity_air_supply(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_air_supply",
            "entity_air_supply",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The most air the entity holds (`Entity.getMaxAirSupply`): 300, a
    /// dolphin's 4800, an axolotl's 6000.
    fn entity_max_air_supply(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_max_air_supply",
            "entity_max_air_supply",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the mob makes no sounds (`Entity.isSilent`, its `Silent`
    /// tag); false for a player.
    fn entity_is_silent(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_silent",
            "entity_is_silent",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Returns the mob's persistent glowing tag (`Entity.hasGlowingTag`).
    fn entity_glowing(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_glowing",
            "entity_glowing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Returns the mob's no-gravity flag (`Entity.isNoGravity`).
    fn entity_no_gravity(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_no_gravity",
            "entity_no_gravity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Returns the entity's frozen ticks (`Entity.getTicksFrozen`).
    fn entity_ticks_frozen(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_ticks_frozen",
            "entity_ticks_frozen",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Ticks the entity has lived (`Entity.tickCount`): mobs, item entities,
    /// experience orbs and minecarts. Wraps as vanilla's `int` does.
    fn entity_tick_count(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_tick_count",
            "entity_tick_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity's model is hidden (`Entity.isInvisible`): the
    /// invisibility effect on a player or a mob, a mob's or an armor stand's
    /// flag.
    fn entity_is_invisible(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_invisible",
            "entity_is_invisible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entity's pose (`Entity.getPose`) as `Pose` names it: `standing`,
    /// `crouching`, `swimming`, `sleeping`, a warden's `roaring`... Kinds
    /// with no pose stand.
    fn entity_pose(&self, entity: EntityRef) -> Option<&'static str> {
        crate::phase::unprovided(
            "Backend::entity_pose",
            "entity_pose",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_state` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityStateBackendMut {
    /// Side: server.
    /// Sets the remaining fire ticks (`Entity.setRemainingFireTicks`).
    fn entity_set_remaining_fire_ticks(
        &mut self,
        entity: EntityRef,
        ticks: i32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_remaining_fire_ticks",
            "entity_set_remaining_fire_ticks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Clears positive fire ticks while retaining fire immunity (`Entity.clearFire`).
    fn entity_clear_fire(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_clear_fire",
            "entity_clear_fire",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the entity's air (`Entity.setAirSupply`).
    fn entity_set_air_supply(&mut self, entity: EntityRef, air: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_air_supply",
            "entity_set_air_supply",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Silences the mob or lets it make sounds again (`Entity.setSilent`).
    /// False for the other kinds.
    fn entity_set_silent(&mut self, entity: EntityRef, silent: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_silent",
            "entity_set_silent",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the mob's persistent glowing tag (`Entity.setGlowingTag`). Returns false for players and guests.
    fn entity_set_glowing(&mut self, entity: EntityRef, glowing: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_glowing",
            "entity_set_glowing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the mob's no-gravity flag (`Entity.setNoGravity`). Returns false for players and guests.
    fn entity_set_no_gravity(&mut self, entity: EntityRef, no_gravity: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_no_gravity",
            "entity_set_no_gravity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the entity's frozen ticks (`Entity.setTicksFrozen`).
    fn entity_set_ticks_frozen(&mut self, entity: EntityRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_ticks_frozen",
            "entity_set_ticks_frozen",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the entity's shared invisibility flag (`Entity.setInvisible`). Returns false for players and guests.
    fn entity_set_invisible(&mut self, entity: EntityRef, invisible: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_invisible",
            "entity_set_invisible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
