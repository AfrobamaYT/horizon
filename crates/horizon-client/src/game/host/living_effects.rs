//! The game's side of the `living_effects` area (`horizon-mod-api/src/host/living_effects.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl LivingEffectsBackend for Game<'_> {
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
}
impl LivingEffectsClientBackend for Game<'_> {
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
impl LivingEffectsBackendMut for Game<'_> {
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
}
