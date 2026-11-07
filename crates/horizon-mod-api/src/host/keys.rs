//! The `keys` area: host methods the game implements (`game/host/keys.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `keys` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait KeysBackend {
    /// Side: both.
    /// The canonical block id retained by the load-owned registration slot. An invalid slot
    /// is a boundary error, including when a wasm guest supplies it.
    fn block_key_id(&self, key: BlockKey) -> String {
        crate::phase::unprovided(
            "Backend::block_key_id",
            "block_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical item id retained by the load-owned registration slot.
    fn item_key_id(&self, key: ItemKey) -> String {
        crate::phase::unprovided(
            "Backend::item_key_id",
            "item_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical effect id retained by the load-owned registration slot.
    fn effect_key_id(&self, key: EffectKey) -> String {
        crate::phase::unprovided(
            "Backend::effect_key_id",
            "effect_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical block entity type id retained by the load-owned slot.
    fn block_entity_type_key_id(&self, key: BlockEntityTypeKey) -> String {
        crate::phase::unprovided(
            "Backend::block_entity_type_key_id",
            "block_entity_type_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
