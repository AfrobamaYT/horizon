//! The `block_entities` area: host methods the game implements (`game/host/block_entities.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `block_entities` area, implemented by the game and reached through [`Host`].
pub trait BlockEntitiesHost {
    /// Side: both.
    /// Find a block entity in a loaded chunk of an open level, or on a client in
    /// the level it shows. An entity taken out for its tick cannot be read in that tick.
    fn block_entity(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockEntityRef> {
        crate::phase::unprovided(
            "Host::block_entity",
            "block_entity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `block_entities` area, implemented by the game and reached through [`ServerHost`].
pub trait BlockEntitiesServerHost {
    /// Side: server.
    /// Sends clients the block at `pos` and its block entity's update tag
    /// (`native_block_entity_update_tag`) again, as `Level.sendBlockUpdated`
    /// does: with the level's other block changes, to the players who see its
    /// chunk, once however often it is called before then. Changes made in a
    /// block entity's tick go out at the start of the next tick, as vanilla's
    /// do (`ServerLevel.tick` broadcasts chunks before it ticks block
    /// entities). Call it when what clients show changed (a machine started),
    /// not for every tick's progress: a menu's data slots carry that. Nothing
    /// happens for an unloaded block.
    fn block_entity_sync(&mut self, pos: [i32; 3]) {
        crate::phase::unprovided(
            "ServerHost::block_entity_sync",
            "block_entity_sync",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `block_entities` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BlockEntitiesBackend {
    /// Side: both.
    /// The block entity's type (`BlockEntity.getType`).
    fn block_entity_kind(&self, entity: BlockEntityRef) -> Option<BlockEntityTypeKey> {
        crate::phase::unprovided(
            "Backend::block_entity_kind",
            "block_entity_kind",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The block entity's position.
    fn block_entity_position(&self, entity: BlockEntityRef) -> Option<[i32; 3]> {
        crate::phase::unprovided(
            "Backend::block_entity_position",
            "block_entity_position",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The block entity's level.
    fn block_entity_level(&self, entity: BlockEntityRef) -> Option<LevelRef> {
        crate::phase::unprovided(
            "Backend::block_entity_level",
            "block_entity_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The data saved without identity or position metadata
    /// (`BlockEntity.saveWithoutMetadata`).
    fn block_entity_data(&self, entity: BlockEntityRef) -> Option<Compound> {
        crate::phase::unprovided(
            "Backend::block_entity_data",
            "block_entity_data",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The vanilla lock's item predicate, if present on a loaded block entity.
    fn block_entity_lock(&self, entity: BlockEntityRef) -> Option<Compound> {
        crate::phase::unprovided(
            "Backend::block_entity_lock",
            "block_entity_lock",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The custom name (`Nameable.getCustomName`) as plain text. None when the block entity
    /// has none or isn't nameable (chests, barrels, furnaces, hoppers, brewing stands,
    /// banners, beacons, enchanting tables, command blocks...), or for a stale handle or a
    /// client.
    fn block_entity_name(&self, entity: BlockEntityRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::block_entity_name",
            "block_entity_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The players whose open menu holds this block entity's container (for openers'
    /// containers, `ContainerOpenersCounter.getEntitiesWithContainerOpen`'s players), from the menus the opener counts read
    /// (`tick_mod_container_viewers`, `ChestBlockEntity$1.isOwnContainer`). Guests in
    /// its level, not spectators nor fake players, then the acting player for a menu
    /// their action opened, which the hub records when the action ends. None for a
    /// stale handle or a client.
    fn block_entity_viewers(&self, entity: BlockEntityRef) -> Option<Vec<PlayerRef>> {
        crate::phase::unprovided(
            "Backend::block_entity_viewers",
            "block_entity_viewers",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// A snapshot of a mod block entity's menu data slots, on the server or in the
    /// client's copy. None for a stale/missing/vanilla entity. Item storage is separate.
    fn block_entity_data_slots(&self, entity: BlockEntityRef) -> Option<Vec<i32>> {
        crate::phase::unprovided(
            "Backend::block_entity_data_slots",
            "block_entity_data_slots",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Read a configured mod holder's declared internal item slot, without automation policy.
    /// Removed, replaced or reloaded holders return BlockEntityItemError::Stale.
    fn block_entity_item_slot(
        &self,
        entity: BlockEntityRef,
        slot: u32,
    ) -> Result<Option<ItemStack>, BlockEntityItemError> {
        crate::phase::unprovided(
            "Backend::block_entity_item_slot",
            "block_entity_item_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' writes of the `block_entities` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BlockEntitiesBackendMut {
    /// Side: server.
    /// Change the lock through the same storage used by container opening and saving.
    fn block_entity_set_lock(&mut self, entity: BlockEntityRef, lock: Option<Compound>) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_lock",
            "block_entity_set_lock",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Merges `data` into the block entity's saved data and loads it, as `/data merge block`
    /// does (`BlockDataAccessor.setData`: `loadWithComponents`, `setChanged`, then
    /// `sendBlockUpdated`, so comparators re-read it and clients get the update). `id`, `x`,
    /// `y` and `z` in `data` are ignored. True when the data changed; false for a stale
    /// handle, a client, another level than the one in place, data nested too deep or that
    /// changes nothing, or a result the block entity can't load. Inside a transaction an
    /// abort puts the block entity, its name, `lock`, loot table roll and attachments back.
    fn block_entity_merge_data(&mut self, entity: BlockEntityRef, data: Compound) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_merge_data",
            "block_entity_merge_data",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Replaces the saved payload, retaining identity and position metadata.
    /// Uses the same transactional load and client update path as `merge_data`.
    /// False for a missing or stale block entity, unsupported data, or no change.
    fn block_entity_replace_data(&mut self, entity: BlockEntityRef, data: Compound) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_replace_data",
            "block_entity_replace_data",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BlockEntity.setChanged`: the chunk is saved again and comparators re-read the
    /// container (`updateNeighbourForOutputSignal`). Inside a transaction it runs once at the
    /// root commit. False for a stale handle, a client or another level than the one in place.
    fn block_entity_set_changed(&mut self, entity: BlockEntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_changed",
            "block_entity_set_changed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the custom name, or clears it with None, as an anvil-renamed item placed with
    /// `components` sets `CustomName`. Plain text only. Clients get a block entity update.
    /// False for a block entity that isn't nameable, a stale handle, a client or another
    /// level than the one in place. Inside a transaction an abort puts the name back.
    fn block_entity_set_name(&mut self, entity: BlockEntityRef, name: Option<&str>) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_name",
            "block_entity_set_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Set a mod block entity's menu data slot in place, joining the current transaction
    /// and its setChanged notice. False for a stale/client/parked/vanilla entity, an
    /// invalid slot or no change. Does not resize the mod's declared data slots.
    fn block_entity_set_data_slot(
        &mut self,
        entity: BlockEntityRef,
        slot: u32,
        value: i32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_data_slot",
            "block_entity_set_data_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sizes a mod block entity's menu data explicitly, using the same write
    /// target, journal and Changed notice as scalar data-slot writes.
    /// Removed, replaced or reloaded holders return BlockEntityDataError::Stale.
    fn block_entity_set_data_slots(
        &mut self,
        entity: BlockEntityRef,
        values: Vec<i32>,
    ) -> Result<bool, BlockEntityDataError> {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_data_slots",
            "block_entity_set_data_slots",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Replace internal storage under the existing transaction; external ITEMS routing is separate.
    /// Removed, replaced or reloaded holders return BlockEntityItemError::Stale.
    fn block_entity_set_item_slot(
        &mut self,
        entity: BlockEntityRef,
        tx: &Transaction,
        slot: u32,
        stack: Option<ItemStack>,
    ) -> Result<bool, BlockEntityItemError> {
        crate::phase::unprovided(
            "BackendMut::block_entity_set_item_slot",
            "block_entity_set_item_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
