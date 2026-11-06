//! The game's side of the `living_refs` area (`horizon-mod-api/src/host/living_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LivingRefsHost for Game<'_> {
    fn living(&self, entity: EntityRef) -> Option<LivingRef> {
        mod_api::__private::check_access(
            "living",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living(entity)
    }
}
impl LivingRefsBackend for Game<'_> {
    fn living_attribute_value(&self, living: LivingRef, id: &str) -> Option<f64> {
        mod_api::__private::check_access(
            "living_attribute_value",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_attribute_value(living, id)
    }
    fn living_attribute_base(&self, living: LivingRef, id: &str) -> Option<f64> {
        mod_api::__private::check_access(
            "living_attribute_base",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_attribute_base(living, id)
    }
    fn living_attributes(&self, living: LivingRef) -> Option<Vec<Identifier>> {
        mod_api::__private::check_access(
            "living_attributes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_attributes(living)
    }
    fn living_attribute_modifiers(
        &self,
        living: LivingRef,
        id: &str,
    ) -> Option<Vec<AttributeModifier>> {
        mod_api::__private::check_access(
            "living_attribute_modifiers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_attribute_modifiers(living, id)
    }
    fn living_equipment(
        &self,
        living: LivingRef,
        slot: EquipmentSlot,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "living_equipment",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_equipment(living, slot)
    }
    fn living_hand_item(&self, living: LivingRef, hand: Hand) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "living_hand_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_hand_item(living, hand)
    }
    fn living_is_tamed(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_tamed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_tamed(living)
    }
    fn living_owner(&self, living: LivingRef) -> Option<u128> {
        mod_api::__private::check_access(
            "living_owner",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_owner(living)
    }
}
impl LivingRefsBackendMut for Game<'_> {
    fn living_add_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        modifier: AttributeModifier,
        lifetime: ModifierLifetime,
    ) -> bool {
        mod_api::__private::check_access(
            "living_add_attribute_modifier",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_add_attribute_modifier(living, attribute, modifier, lifetime)
    }
    fn living_replace_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        modifier: AttributeModifier,
        lifetime: ModifierLifetime,
    ) -> bool {
        mod_api::__private::check_access(
            "living_replace_attribute_modifier",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_replace_attribute_modifier(
            living,
            attribute,
            modifier,
            lifetime,
        )
    }
    fn living_remove_attribute_modifier(
        &mut self,
        living: LivingRef,
        attribute: &str,
        id: &Identifier,
    ) -> bool {
        mod_api::__private::check_access(
            "living_remove_attribute_modifier",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_remove_attribute_modifier(living, attribute, id)
    }
    fn living_set_attribute_base(
        &mut self,
        living: LivingRef,
        attribute: &str,
        base: f64,
    ) -> bool {
        mod_api::__private::check_access(
            "living_set_attribute_base",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_attribute_base(living, attribute, base)
    }
    fn living_set_equipment(
        &mut self,
        living: LivingRef,
        slot: EquipmentSlot,
        stack: Option<ItemStack>,
    ) -> bool {
        mod_api::__private::check_access(
            "living_set_equipment",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_equipment(living, slot, stack)
    }
    fn living_set_hand_item(
        &mut self,
        living: LivingRef,
        hand: Hand,
        stack: Option<ItemStack>,
    ) -> bool {
        mod_api::__private::check_access(
            "living_set_hand_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_hand_item(living, hand, stack)
    }
    fn living_hurt_and_break(
        &mut self,
        living: LivingRef,
        slot: EquipmentSlot,
        amount: i32,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "living_hurt_and_break",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_hurt_and_break(living, slot, amount)
    }
    fn living_drop(
        &mut self,
        living: LivingRef,
        stack: ItemStack,
        throw_randomly: bool,
        include_thrower_name: bool,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "living_drop",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_drop(living, stack, throw_randomly, include_thrower_name)
    }
}
