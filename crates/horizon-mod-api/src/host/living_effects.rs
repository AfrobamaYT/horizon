//! The `living_effects` area: host methods the game implements (`game/host/living_effects.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_effects` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingEffectsBackend {
    /// Side: both.
    /// The entity's instance of `effect` (`LivingEntity.getEffect`): its id
    /// namespaced as the registry has it, the ticks left (-1 infinite), the
    /// amplifier and the flags. The effect is a registration-slot key resolved at load.
    /// None when it has the effect not, before effect resolution, or for an armor
    /// stand, the dragon, a stale handle or a client.
    fn living_effect(&self, living: LivingRef, effect: EffectKey) -> Option<NativeActiveEffect> {
        crate::phase::unprovided(
            "Backend::living_effect",
            "living_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The entity's active effects (`LivingEntity.getActiveEffects`), in the
    /// order their store keeps them, in `Some` even when it has none. None
    /// for an armor stand, the dragon, a stale handle or a client.
    fn living_effects(&self, living: LivingRef) -> Option<Vec<NativeActiveEffect>> {
        crate::phase::unprovided(
            "Backend::living_effects",
            "living_effects",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity has `effect` (`LivingEntity.hasEffect`), using its resolved key.
    /// False before effect resolution, for an armor stand, the dragon,
    /// a stale handle or a client.
    fn living_potion_affected(&self, living: LivingRef, effect: EffectKey) -> bool {
        crate::phase::unprovided(
            "Backend::living_potion_affected",
            "living_potion_affected",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' client-side reads of the `living_effects` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingEffectsClientBackend {
    /// Side: client.
    /// `LivingEntity.getEffectBlendFactor(effect, partialTick)` on a client:
    /// how far the effect has faded in, from 0 to 1, between the last tick
    /// and the next at `partial`. Darkness fades in and out over 22 ticks and
    /// nausea in over 150 and out over 20 (`MobEffect.setBlendDuration`), the
    /// fades the fog, lightmap and screen spin draw. Other effects start at
    /// 0 after a blended add and interpolate to their post-countdown target
    /// during each client tick: 1 while active, 0 once expired. An unblended
    /// add starts at its target immediately; an update preserves the old blend.
    /// `partial` is not clamped. The registration-slot key is resolved at load.
    /// 0 when the entity lacks the effect. None before effect resolution, for a stale
    /// handle, or an entity other than the local player: the client keeps no
    /// other entity's effects.
    fn living_effect_blend_factor(
        &self,
        living: LivingRef,
        effect: EffectKey,
        partial: f32,
    ) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::living_effect_blend_factor",
            "living_effect_blend_factor",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_effects` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingEffectsBackendMut {
    /// Side: server.
    /// Apply one loaded consumable effect without consuming the item.
    fn living_apply_consume_effect(
        &mut self,
        living: LivingRef,
        stack: &ItemStack,
        index: usize,
    ) -> Option<bool> {
        crate::phase::unprovided(
            "BackendMut::living_apply_consume_effect",
            "living_apply_consume_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.addEffect(instance)`: gives the living entity the
    /// effect by the game's own path. A player's or a guest's goes through
    /// `add_player_effect` (so `MobEffectInstance.update`'s rules, the
    /// attribute modifiers, absorption and the added-effect hooks of a
    /// content mod's effect all apply); a mob's goes past `canBeAffected`
    /// (a spider shrugs off poison) into its effects with the same hooks.
    /// The amplifier is cut to 0..=255 and the duration is -1 (infinite) or
    /// ticks. False before effect resolution, for a duration below -1, an armor
    /// stand, the dragon, a stale handle or a client, or when the active
    /// instance did not change, as in vanilla's `addEffect`.
    fn living_add_effect(&mut self, living: LivingRef, effect: &NativeActiveEffect) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_add_effect",
            "living_add_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.removeEffect(effect)` the way its running out goes
    /// (`LivingEntity.tickEffects` drops it): what the effect gave the
    /// player goes with it — the attribute modifiers recompute
    /// (`apply_effect_status`) and absorption's hearts leave — and a mob's
    /// hidden and ambient lists drop it too, with absorption's hearts
    /// leaving the same way its expiry takes them. False (and nothing
    /// changes) when the entity has the effect not, before effect resolution, or for
    /// an armor stand, the dragon, a stale handle or a client.
    fn living_remove_effect(&mut self, living: LivingRef, effect: EffectKey) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_remove_effect",
            "living_remove_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.removeAllEffects()` the way their running out goes:
    /// every effect's attribute modifiers and absorption hearts leave with
    /// them. False when the entity has none, or for an armor stand, the
    /// dragon, a stale handle or a client.
    fn living_remove_all_effects(&mut self, living: LivingRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_remove_all_effects",
            "living_remove_all_effects",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
