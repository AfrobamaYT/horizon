//! The game's side of the `entity_data` area (`horizon-mod-api/src/host/entity_data.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EntityDataBackend for Game<'_> {
    fn entity_tags(&self, entity: EntityRef) -> Option<Vec<String>> {
        mod_api::__private::check_access(
            "entity_tags",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_tags(entity)
    }
    fn entity_name(&self, entity: EntityRef) -> Option<String> {
        mod_api::__private::check_access(
            "entity_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_name(entity)
    }
    fn entity_custom_name_visible(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_custom_name_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_custom_name_visible(entity)
    }
    fn entity_classification(&self, entity: EntityRef) -> Option<MobCategory> {
        mod_api::__private::check_access(
            "entity_classification",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_classification(entity)
    }
    fn entity_added_to_level(&self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_added_to_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_added_to_level(entity)
    }
}
impl EntityDataBackendMut for Game<'_> {
    fn entity_nbt(&mut self, entity: EntityRef) -> Option<Compound> {
        mod_api::__private::check_access(
            "entity_nbt",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_nbt(entity)
    }
    fn entity_merge_nbt(&mut self, entity: EntityRef, nbt: Compound) -> bool {
        mod_api::__private::check_access(
            "entity_merge_nbt",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_merge_nbt(entity, nbt)
    }
    fn entity_add_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_add_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_add_tag(entity, tag)
    }
    fn entity_remove_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_remove_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_remove_tag(entity, tag)
    }
    fn entity_set_custom_name(&mut self, entity: EntityRef, name: Option<Text>) -> bool {
        mod_api::__private::check_access(
            "entity_set_custom_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_custom_name(entity, name)
    }
    fn entity_set_custom_name_visible(
        &mut self,
        entity: EntityRef,
        visible: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_set_custom_name_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_custom_name_visible(entity, visible)
    }
}
