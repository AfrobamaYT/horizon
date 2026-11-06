//! The game's side of the `entity_remove` area (`horizon-mod-api/src/host/entity_remove.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EntityRemoveBackendMut for Game<'_> {
    fn entity_discard(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_discard",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_discard(entity)
    }
    fn entity_remove(&mut self, entity: EntityRef, reason: NativeRemovalReason) -> bool {
        mod_api::__private::check_access(
            "entity_remove",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_remove(entity, reason)
    }
    fn entity_spawn_at_location(
        &mut self,
        entity: EntityRef,
        stack: ItemStack,
        y_offset: f32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_spawn_at_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_spawn_at_location(entity, stack, y_offset)
    }
}
