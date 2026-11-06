//! The `living_refs` area: host methods the game implements (`game/host/living_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `living_refs` area, implemented by the game and reached through [`Host`].
pub trait LivingRefsHost {
    /// Side: both.
    /// The entity as a living entity (vanilla's `LivingEntity`: a mob, a
    /// player, an armor stand or the dragon), or None for another kind or a
    /// stale handle.
    ///
    /// On a client, only the local player is one: the client keeps the others'
    /// equipment, effects and health only as far as it draws them. API 382.
    fn living(&self, entity: EntityRef) -> Option<LivingRef> {
        crate::phase::unprovided(
            "Host::living",
            "living",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `living_refs` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingRefsBackend {
    /// Side: both.
    /// The effective value of an attribute on a living entity
    /// (`AttributeMap.getValue`). None for an unknown or absent attribute, an
    /// unavailable entity, a stale handle, or on a client for any entity but
    /// the local player, whose copy of the synced instances it reads. A mob,
    /// an armor stand and the dragon answer from their live attribute map: the
    /// type's defaults, the saved `attributes`, and the modifiers of what they
    /// wear or hold.
    fn living_attribute_value(&self, living: LivingRef, id: &str) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::living_attribute_value",
            "living_attribute_value",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The base value of an attribute, before modifiers
    /// (`AttributeMap.getBaseValue`), read as the effective value is.
    fn living_attribute_base(&self, living: LivingRef, id: &str) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::living_attribute_base",
            "living_attribute_base",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The ids of the attributes the entity's type has
    /// (`AttributeMap`'s holder's attribute set), in registry order: the same
    /// set the value and base reads answer for. None for a stale handle or an
    /// unavailable entity; on a client, for any entity but the local player
    /// (the client keeps no attribute map of another entity's, only the few
    /// fields it draws).
    fn living_attributes(&self, living: LivingRef) -> Option<Vec<Identifier>> {
        crate::phase::unprovided(
            "Backend::living_attributes",
            "living_attributes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The modifiers on the entity's attribute `id`
    /// (`AttributeInstance.getModifiers`), in the order they were added: a
    /// mod's transient and permanent ones, the saved list's, and what the
    /// worn and held items grant. An attribute no modifier is on answers an
    /// empty list; None for an attribute the entity type does not have, a
    /// stale handle, or on a client for any entity but the local player,
    /// whose copy of the synced instances it reads.
    fn living_attribute_modifiers(
        &self,
        living: LivingRef,
        id: &str,
    ) -> Option<Vec<AttributeModifier>> {
        crate::phase::unprovided(
            "Backend::living_attribute_modifiers",
            "living_attribute_modifiers",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The stack in `slot` (`LivingEntity.getItemBySlot`). Players, guests,
    /// mobs, armor stands and the dragon store all eight slots. An armor
    /// stand's body and saddle are storage only: vanilla's `canUseSlot`
    /// refuses them for interaction. None means empty, unsupported or stale.
    /// The stack in equipment slot `slot` (`LivingEntity.getItemBySlot`):
    /// None when it is empty, the entity has no such slot (a player has no
    /// body or saddle slot), or the handle is stale. API 382.
    fn living_equipment(&self, living: LivingRef, slot: EquipmentSlot) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::living_equipment",
            "living_equipment",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The stack in hand `hand` (`LivingEntity.getItemInHand`). API 382.
    fn living_hand_item(&self, living: LivingRef, hand: Hand) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::living_hand_item",
            "living_hand_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TamableAnimal.isTame`, `AbstractHorse.isTamed`: None for a kind that
    /// can't be tamed or a stale handle. API @API:living_refs@.
    fn living_is_tamed(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_is_tamed",
            "living_is_tamed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The owner's UUID (`OwnableEntity.getOwnerReference`): None for a wild
    /// mob, a kind that can't be tamed, or a stale handle. API @API:living_refs@.
    fn living_owner(&self, living: LivingRef) -> Option<u128> {
        crate::phase::unprovided(
            "Backend::living_owner",
            "living_owner",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_refs` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingRefsBackendMut {
    /// Side: server.
    /// Adds an attribute modifier (`AttributeInstance.addTransientModifier`
    /// or `addPermanentModifier`). A permanent one is saved with the entity;
    /// a transient one lasts until it unloads. It goes where `/attribute`
    /// writes (`Game::write_attribute`), so gameplay and the entity's clients
    /// see it. False when the entity already has a modifier with that id on
    /// the attribute (vanilla throws), for an attribute the entity type does
    /// not have, a non-finite amount, the dragon (whose attributes nothing
    /// reads yet), a stale handle, or on a client.
    fn living_add_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        modifier: AttributeModifier,
        lifetime: ModifierLifetime,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_add_attribute_modifier",
            "living_add_attribute_modifier",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Adds an attribute modifier in place of the one with the same id, if
    /// any (`addOrUpdateTransientModifier`, `addOrReplacePermanentModifier`):
    /// the new one's lifetime wins. False as `add_attribute_modifier` is,
    /// except that a taken id is replaced.
    fn living_replace_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        modifier: AttributeModifier,
        lifetime: ModifierLifetime,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_replace_attribute_modifier",
            "living_replace_attribute_modifier",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Removes the modifier with id `id` from the attribute
    /// (`AttributeInstance.removeModifier`), whatever its lifetime. False when
    /// there was none, and in the cases `add_attribute_modifier` is.
    fn living_remove_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        id: &Identifier,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_remove_attribute_modifier",
            "living_remove_attribute_modifier",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the attribute's base value (`AttributeInstance.setBaseValue`);
    /// the value stays within the attribute's range. False for a non-finite
    /// base, and in the cases `add_attribute_modifier` is.
    fn living_set_attribute_base(&mut self, living: LivingRef, attribute: &str, base: f64) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_attribute_base",
            "living_set_attribute_base",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Puts `stack` in `slot` (`LivingEntity.setItemSlot`); None, air or a
    /// zero count empties it. Players, guests, mobs, armor stands and the
    /// dragon store all eight slots. An armor stand stores body and saddle
    /// too: `ArmorStand.canUseSlot` only stops interaction, and
    /// `LivingEntity.setItemSlot` keeps whatever it is given. False means
    /// unsupported or stale.
    /// This is a direct write: it takes effect and is final immediately, even
    /// inside `host.transaction`. A mob's write makes any equip/unequip event
    /// and equip sound immediately; a stand's or the dragon's makes neither. API 382.
    fn living_set_equipment(
        &mut self,
        living: LivingRef,
        slot: EquipmentSlot,
        stack: Option<ItemStack>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_equipment",
            "living_set_equipment",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Puts `stack` in hand `hand` (`LivingEntity.setItemInHand`): the main
    /// hand's slot or the off hand's, written as `set_equipment`
    /// writes them, with its equip events and sound. None, air or a zero count
    /// empties the hand. False means unsupported or stale.
    fn living_set_hand_item(
        &mut self,
        living: LivingRef,
        hand: Hand,
        stack: Option<ItemStack>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_hand_item",
            "living_set_hand_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Damage an equipped stack without an equip/unequip notification.
    fn living_hurt_and_break(
        &mut self,
        living: LivingRef,
        slot: EquipmentSlot,
        amount: i32,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::living_hurt_and_break",
            "living_hurt_and_break",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Throws `stack`, which the mod holds, from the eyes (`LivingEntity.drop`,
    /// `ServerPlayer.drop`), never through a slot: along the view with a
    /// main-hand swing, or with `throw_randomly` scattered with none, as a
    /// death's drops. `include_thrower_name` sets the thrower, and a player's
    /// throw along the view then counts in its stats and fires `item_toss`.
    /// None for an empty stack, a stand or the dragon, a stale handle, or an
    /// item a mod kept out. API @API:living_refs@.
    fn living_drop(
        &mut self,
        living: LivingRef,
        stack: ItemStack,
        throw_randomly: bool,
        include_thrower_name: bool,
    ) -> Option<EntityRef> {
        crate::phase::unprovided(
            "BackendMut::living_drop",
            "living_drop",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
