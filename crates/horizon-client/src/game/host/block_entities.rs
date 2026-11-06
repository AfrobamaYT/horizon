//! The game's side of the `block_entities` area (`horizon-mod-api/src/host/block_entities.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl BlockEntitiesHost for Game<'_> {
    fn block_entity(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockEntityRef> {
        mod_api::__private::check_access("block_entity", mod_api::__private::LoadAccess::WorldOnly);
        self.native_block_entity_ref(level, pos)
    }
}
impl BlockEntitiesServerHost for Game<'_> {
    fn block_entity_sync(&mut self, pos: [i32; 3]) {
        mod_api::__private::check_access(
            "block_entity_sync",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_sync(pos)
    }
}
impl BlockEntitiesBackend for Game<'_> {
    fn block_entity_kind(&self, entity: BlockEntityRef) -> Option<BlockEntityTypeKey> {
        mod_api::__private::check_access(
            "block_entity_kind",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_kind(entity)
    }
    fn block_entity_position(&self, entity: BlockEntityRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "block_entity_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_position(entity)
    }
    fn block_entity_level(&self, entity: BlockEntityRef) -> Option<LevelRef> {
        mod_api::__private::check_access(
            "block_entity_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_level(entity)
    }
    fn block_entity_data(&self, entity: BlockEntityRef) -> Option<Compound> {
        mod_api::__private::check_access(
            "block_entity_data",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_data(entity)
    }
    fn block_entity_lock(&self, entity: BlockEntityRef) -> Option<Compound> {
        mod_api::__private::check_access(
            "block_entity_lock",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_lock(entity)
    }
    fn block_entity_name(&self, entity: BlockEntityRef) -> Option<String> {
        mod_api::__private::check_access(
            "block_entity_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_name(entity)
    }
    fn block_entity_viewers(&self, entity: BlockEntityRef) -> Option<Vec<PlayerRef>> {
        mod_api::__private::check_access(
            "block_entity_viewers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_viewers(entity)
    }
    fn block_entity_data_slots(&self, entity: BlockEntityRef) -> Option<Vec<i32>> {
        mod_api::__private::check_access(
            "block_entity_data_slots",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_data_slots(entity)
    }
    fn block_entity_item_slot(
        &self,
        entity: BlockEntityRef,
        slot: u32,
    ) -> Result<Option<ItemStack>, BlockEntityItemError> {
        mod_api::__private::check_access(
            "block_entity_item_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_item_slot(entity, slot)
    }
}
impl BlockEntitiesBackendMut for Game<'_> {
    fn block_entity_set_lock(&mut self, entity: BlockEntityRef, lock: Option<Compound>) -> bool {
        mod_api::__private::check_access(
            "block_entity_set_lock",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_lock(entity, lock)
    }
    fn block_entity_merge_data(&mut self, entity: BlockEntityRef, data: Compound) -> bool {
        mod_api::__private::check_access(
            "block_entity_merge_data",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_merge_data(entity, data)
    }
    fn block_entity_replace_data(&mut self, entity: BlockEntityRef, data: Compound) -> bool {
        mod_api::__private::check_access(
            "block_entity_replace_data",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_replace_data(entity, data)
    }
    fn block_entity_set_changed(&mut self, entity: BlockEntityRef) -> bool {
        mod_api::__private::check_access(
            "block_entity_set_changed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_changed(entity)
    }
    fn block_entity_set_name(&mut self, entity: BlockEntityRef, name: Option<&str>) -> bool {
        mod_api::__private::check_access(
            "block_entity_set_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_name(entity, name)
    }
    fn block_entity_set_data_slot(
        &mut self,
        entity: BlockEntityRef,
        slot: u32,
        value: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "block_entity_set_data_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_data_slot(entity, slot, value)
    }
    fn block_entity_set_data_slots(
        &mut self,
        entity: BlockEntityRef,
        values: Vec<i32>,
    ) -> Result<bool, BlockEntityDataError> {
        mod_api::__private::check_access(
            "block_entity_set_data_slots",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_data_slots(entity, values)
    }
    fn block_entity_set_item_slot(
        &mut self,
        entity: BlockEntityRef,
        tx: &Transaction,
        slot: u32,
        stack: Option<ItemStack>,
    ) -> Result<bool, BlockEntityItemError> {
        mod_api::__private::check_access(
            "block_entity_set_item_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_set_item_slot(entity, tx, slot, stack)
    }
}
