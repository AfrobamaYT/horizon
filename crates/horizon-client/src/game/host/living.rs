//! The game's side of the `living` area (`horizon-mod-api/src/host/living.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LivingHost for Game<'_> {
    fn living(&self, entity: EntityRef) -> Option<LivingRef> {
        mod_api::__private::check_access("living", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living(entity)
    }
}
impl LivingBackend for Game<'_> {
    fn living_effect(&self, living: LivingRef, effect: EffectKey) -> Option<NativeActiveEffect> {
        mod_api::__private::check_access(
            "living_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_effect(living, effect)
    }
    fn living_effects(&self, living: LivingRef) -> Option<Vec<NativeActiveEffect>> {
        mod_api::__private::check_access(
            "living_effects",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_effects(living)
    }
    fn living_potion_affected(&self, living: LivingRef, effect: EffectKey) -> bool {
        mod_api::__private::check_access(
            "living_potion_affected",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_potion_affected(living, effect)
    }
    fn living_gliding_ticks(&self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_gliding_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_gliding_ticks(living)
    }
    fn living_is_gliding(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_gliding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_gliding(living)
    }
    fn living_is_sleeping(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_sleeping(living)
    }
    fn living_sleeping_pos(&self, living: LivingRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "living_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_sleeping_pos(living)
    }
    fn living_is_jumping(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_jumping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_jumping(living)
    }
    fn living_is_climbing(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_climbing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_climbing(living)
    }
    fn living_baby(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access("living_baby", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_baby(living)
    }
    fn living_health(&self, living: LivingRef) -> Option<f32> {
        mod_api::__private::check_access(
            "living_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_health(living)
    }
    fn living_max_health(&self, living: LivingRef) -> Option<f32> {
        mod_api::__private::check_access(
            "living_max_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_max_health(living)
    }
    fn living_alive(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access("living_alive", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_alive(living)
    }
    fn living_kill_credit(&self, living: LivingRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "living_kill_credit",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_kill_credit(living)
    }
    fn living_last_attacker(&self, living: LivingRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "living_last_attacker",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_last_attacker(living)
    }
    fn living_has_line_of_sight(&self, living: LivingRef, target: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_has_line_of_sight",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_has_line_of_sight(living, target)
    }
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
    fn living_equipment(&self, living: LivingRef, slot: EquipmentSlot) -> Option<ItemStack> {
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
        mod_api::__private::check_access("living_owner", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_owner(living)
    }
    fn living_item_use_remaining(&self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_item_use_remaining",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_item_use_remaining(living)
    }
    fn living_used_item_hand(&self, living: LivingRef) -> Option<Hand> {
        mod_api::__private::check_access(
            "living_used_item_hand",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_used_item_hand(living)
    }
    fn living_dimensions_for_pose(&self, living: LivingRef, pose: Pose) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "living_dimensions_for_pose",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_dimensions_for_pose(living, pose)
    }
}
impl LivingClientBackend for Game<'_> {
    fn living_effect_blend_factor(
        &self,
        living: LivingRef,
        effect: EffectKey,
        partial: f32,
    ) -> Option<f32> {
        mod_api::__private::check_access(
            "living_effect_blend_factor",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_effect_blend_factor(living, effect, partial)
    }
}
impl LivingBackendMut for Game<'_> {
    fn living_apply_consume_effect(
        &mut self,
        living: LivingRef,
        stack: &ItemStack,
        index: usize,
    ) -> Option<bool> {
        mod_api::__private::check_access(
            "living_apply_consume_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_apply_consume_effect(living, stack, index)
    }
    fn living_add_effect(&mut self, living: LivingRef, effect: &NativeActiveEffect) -> bool {
        mod_api::__private::check_access(
            "living_add_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_add_effect(living, effect)
    }
    fn living_remove_effect(&mut self, living: LivingRef, effect: EffectKey) -> bool {
        mod_api::__private::check_access(
            "living_remove_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_remove_effect(living, effect)
    }
    fn living_remove_all_effects(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_remove_all_effects",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_remove_all_effects(living)
    }
    fn living_stop_gliding(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_stop_gliding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stop_gliding(living)
    }
    fn living_start_sleeping(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "living_start_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_start_sleeping(living, pos)
    }
    fn living_stop_sleeping(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_stop_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stop_sleeping(living)
    }
    fn living_set_sleeping_pos(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "living_set_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_sleeping_pos(living, pos)
    }
    fn living_clear_sleeping_pos(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_clear_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_clear_sleeping_pos(living)
    }
    fn living_set_is_jumping(&mut self, living: LivingRef, jumping: bool) -> bool {
        mod_api::__private::check_access(
            "living_set_is_jumping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_is_jumping(living, jumping)
    }
    fn living_set_health(&mut self, living: LivingRef, health: f32) -> bool {
        mod_api::__private::check_access(
            "living_set_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_health(living, health)
    }
    fn living_heal(&mut self, living: LivingRef, amount: f32) -> bool {
        mod_api::__private::check_access("living_heal", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_heal(living, amount)
    }
    fn living_random_teleport(
        &mut self,
        living: LivingRef,
        at: [f64; 3],
        avoid: Option<&str>,
    ) -> bool {
        mod_api::__private::check_access(
            "living_random_teleport",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_random_teleport(living, at, avoid)
    }
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
        self.native_living_replace_attribute_modifier(living, attribute, modifier, lifetime)
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
    fn living_set_attribute_base(&mut self, living: LivingRef, attribute: &str, base: f64) -> bool {
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
        mod_api::__private::check_access("living_drop", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_drop(living, stack, throw_randomly, include_thrower_name)
    }
    fn living_swing(&mut self, living: LivingRef, hand: Hand, send_to_self: bool) -> bool {
        mod_api::__private::check_access("living_swing", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_swing(living, hand, send_to_self)
    }
    fn living_arrow_count(&mut self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_arrow_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_arrow_count(living)
    }
    fn living_set_arrow_count(&mut self, living: LivingRef, count: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_arrow_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_arrow_count(living, count)
    }
    fn living_stinger_count(&mut self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_stinger_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stinger_count(living)
    }
    fn living_set_stinger_count(&mut self, living: LivingRef, count: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_stinger_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_stinger_count(living, count)
    }
    fn living_set_arrow_removal_ticks(&mut self, living: LivingRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_arrow_removal_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_arrow_removal_ticks(living, ticks)
    }
    fn living_set_stinger_removal_ticks(&mut self, living: LivingRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_stinger_removal_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_stinger_removal_ticks(living, ticks)
    }
    fn living_set_item_use_remaining(&mut self, living: LivingRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_item_use_remaining",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_item_use_remaining(living, ticks)
    }
    fn living_start_using_item(&mut self, living: LivingRef, hand: Hand) -> bool {
        mod_api::__private::check_access(
            "living_start_using_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_start_using_item(living, hand)
    }
    fn living_release_using_item(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_release_using_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_release_using_item(living)
    }
}
