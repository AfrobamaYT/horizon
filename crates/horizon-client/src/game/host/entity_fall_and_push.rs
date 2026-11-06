//! The game's side of the `entity_fall_and_push` area (`horizon-mod-api/src/host/entity_fall_and_push.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EntityFallAndPushBackend for Game<'_> {
    fn minecart_push(&self, cart: MinecartRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "minecart_push",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_push(cart)
    }
}
impl EntityFallAndPushBackendMut for Game<'_> {
    fn entity_reset_fall_distance(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_reset_fall_distance",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_reset_fall_distance(entity)
    }
    fn minecart_set_push(&mut self, cart: MinecartRef, push: [f64; 3]) -> bool {
        mod_api::__private::check_access(
            "minecart_set_push",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_minecart_set_push(cart, push)
    }
}
