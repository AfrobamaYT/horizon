//! The game's side of the `minecart_refs` area (`horizon-mod-api/src/host/minecart_refs.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl MinecartRefsServerHost for Game<'_> {
    fn minecart(&self, entity: EntityRef) -> Option<MinecartRef> {
        mod_api::__private::check_access("minecart", mod_api::__private::LoadAccess::WorldOnly);
        self.native_minecart_ref(entity)
    }
}
impl MinecartRefsBackend for Game<'_> {
    fn minecart_kind(&self, cart: MinecartRef) -> Option<NativeMinecartKind> {
        mod_api::__private::check_access(
            "minecart_kind",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_kind(cart)
    }
    fn minecart_max_speed(&self, cart: MinecartRef) -> Option<f64> {
        mod_api::__private::check_access(
            "minecart_max_speed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_max_speed(cart)
    }
    fn minecart_is_on_rails(&self, cart: MinecartRef) -> Option<bool> {
        mod_api::__private::check_access(
            "minecart_is_on_rails",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_is_on_rails(cart)
    }
    fn minecart_flipped(&self, cart: MinecartRef) -> Option<bool> {
        mod_api::__private::check_access(
            "minecart_flipped",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_flipped(cart)
    }
    fn minecart_damage(&self, cart: MinecartRef) -> Option<f32> {
        mod_api::__private::check_access(
            "minecart_damage",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_damage(cart)
    }
    fn minecart_hurt_time(&self, cart: MinecartRef) -> Option<i32> {
        mod_api::__private::check_access(
            "minecart_hurt_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_hurt_time(cart)
    }
    fn minecart_custom_display_block(&self, cart: MinecartRef) -> Option<Option<BlockStateId>> {
        mod_api::__private::check_access(
            "minecart_custom_display_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_custom_display_block(cart)
    }
    fn minecart_display_offset(&self, cart: MinecartRef) -> Option<i32> {
        mod_api::__private::check_access(
            "minecart_display_offset",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_display_offset(cart)
    }
    fn minecart_fuel(&self, cart: MinecartRef) -> Option<i32> {
        mod_api::__private::check_access(
            "minecart_fuel",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_fuel(cart)
    }
    fn minecart_fuse(&self, cart: MinecartRef) -> Option<i32> {
        mod_api::__private::check_access(
            "minecart_fuse",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_fuse(cart)
    }
    fn minecart_hopper_enabled(&self, cart: MinecartRef) -> Option<bool> {
        mod_api::__private::check_access(
            "minecart_hopper_enabled",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_hopper_enabled(cart)
    }
    fn minecart_command(&self, cart: MinecartRef) -> Option<String> {
        mod_api::__private::check_access(
            "minecart_command",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_command(cart)
    }
}
impl MinecartRefsBackendMut for Game<'_> {
    fn minecart_set_flipped(&mut self, cart: MinecartRef, flipped: bool) -> bool {
        mod_api::__private::check_access(
            "minecart_set_flipped",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_flipped(cart, flipped)
    }
    fn minecart_set_damage(&mut self, cart: MinecartRef, damage: f32) -> bool {
        mod_api::__private::check_access(
            "minecart_set_damage",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_damage(cart, damage)
    }
    fn minecart_set_hurt_time(&mut self, cart: MinecartRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "minecart_set_hurt_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_hurt_time(cart, ticks)
    }
    fn minecart_set_custom_display_block(
        &mut self,
        cart: MinecartRef,
        state: Option<BlockStateId>,
    ) -> bool {
        mod_api::__private::check_access(
            "minecart_set_custom_display_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_custom_display_block(cart, state)
    }
    fn minecart_set_display_offset(&mut self, cart: MinecartRef, offset: i32) -> bool {
        mod_api::__private::check_access(
            "minecart_set_display_offset",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_display_offset(cart, offset)
    }
    fn minecart_set_fuel(&mut self, cart: MinecartRef, fuel: i32) -> bool {
        mod_api::__private::check_access(
            "minecart_set_fuel",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_fuel(cart, fuel)
    }
    fn minecart_prime_fuse(&mut self, cart: MinecartRef) -> bool {
        mod_api::__private::check_access(
            "minecart_prime_fuse",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_prime_fuse(cart)
    }
    fn minecart_set_hopper_enabled(&mut self, cart: MinecartRef, enabled: bool) -> bool {
        mod_api::__private::check_access(
            "minecart_set_hopper_enabled",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_hopper_enabled(cart, enabled)
    }
    fn minecart_set_command(&mut self, cart: MinecartRef, command: &str) -> bool {
        mod_api::__private::check_access(
            "minecart_set_command",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_command(cart, command)
    }
}
