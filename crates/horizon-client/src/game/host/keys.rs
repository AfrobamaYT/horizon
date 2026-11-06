//! The game's side of the `keys` area (`horizon-mod-api/src/host/keys.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl KeysBackend for Game<'_> {
    fn block_key_id(&self, key: BlockKey) -> String {
        mod_api::__private::check_access("block_key_id", mod_api::__private::LoadAccess::WorldOnly);
        self.native_block_key_id(key)
    }
    fn item_key_id(&self, key: ItemKey) -> String {
        mod_api::__private::check_access("item_key_id", mod_api::__private::LoadAccess::WorldOnly);
        self.native_item_key_id(key)
    }
    fn effect_key_id(&self, key: EffectKey) -> String {
        mod_api::__private::check_access(
            "effect_key_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_effect_key_id(key)
    }
    fn block_entity_type_key_id(&self, key: BlockEntityTypeKey) -> String {
        mod_api::__private::check_access(
            "block_entity_type_key_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_type_key_id(key)
    }
}
