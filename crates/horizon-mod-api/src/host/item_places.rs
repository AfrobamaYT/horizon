//! The `item_places` area: host methods the game implements (`game/host/item_places.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `item_places` area, implemented by the game and reached through [`Host`].
pub trait ItemPlacesHost {
    /// Side: both.
    /// `EntityType.is(TagKey)`: whether `entity_type` (`minecraft:player`) is in the entity
    /// type tag `tag` (`minecraft:cannot_be_pushed_onto_boats`).
    fn entity_type_in_tag(&self, entity_type: &str, tag: &str) -> bool {
        crate::phase::unprovided(
            "Host::entity_type_in_tag",
            "entity_type_in_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `item_places` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ItemPlacesBackend {
    /// Side: both.
    /// `ResourceHandler.insert(index, …)`: as `native_item_container_insert`,
    /// into slot `slot` alone: topped up when it holds the same item and
    /// components, filled when empty. Nothing goes into a slot past the
    /// item's slots or holding another item.
    fn item_container_insert_slot(
        &self,
        stack: &ItemStack,
        slot: usize,
        item: &ItemStack,
    ) -> Option<(u32, ItemStack)> {
        crate::phase::unprovided(
            "Backend::item_container_insert_slot",
            "item_container_insert_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ResourceHandler.isValid` and `getCapacityAsLong`: how many of `item`
    /// slot `slot` of one item of `stack` holds when empty. None of a shulker
    /// box, nor into a slot past the item's slots.
    fn item_container_capacity(&self, stack: &ItemStack, slot: usize, item: &ItemStack) -> u32 {
        crate::phase::unprovided(
            "Backend::item_container_capacity",
            "item_container_capacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many slots the inventory of entity `entity` in the level in place has, as
    /// `entity_slots` reaches them: none for an entity without one, and on a client.
    fn entity_container_size(&self, entity: u32) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::entity_container_size",
            "entity_container_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many slots the container entity `entity` of the level in place shows automation
    /// (`EntitySelector.CONTAINER_ENTITY_SELECTOR`: chest and hopper minecarts, chest boats; a
    /// chested mount is not one): none for any other entity, and on a client.
    fn entity_automation_size(&self, entity: u32) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::entity_automation_size",
            "entity_automation_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Inventory.getSelectedSlot`: the hotbar slot the player holds, 0..9.
    fn player_selected_slot(&self, player: PlayerRef) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::player_selected_slot",
            "player_selected_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `item_places` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ItemPlacesBackendMut {
    /// Side: server.
    /// The stack at a player's or an item entity's place; none for an empty one, a player who
    /// isn't here or is dead, an item entity not in the level in place, any other place
    /// (`ItemPlace::stack` reads those), and on a client.
    fn game_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::game_stack",
            "game_stack",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemAccess.insert` at a player's place, an item entity's or an entity's slot: up to
    /// `stack.count` of `stack`, into the place where it's empty or holds the same item and
    /// components, up to the item's stack size. The rest goes where vanilla puts an item split
    /// off it: a player's into their inventory, else dropped as `Player.drop(stack, false)` at
    /// the root commit (a creative player's hand takes anything, and gives them one if they have
    /// none, as `ItemUtils.createFilledResult`); an item entity's dropped beside it at the root
    /// commit; an entity slot's into the entity's other slots (`forHandlerIndex`). How much
    /// went: 0 where `native_game_stack` reads nothing for want of the place. In transaction
    /// `tx`, which keeps each place as it was. The player's screen follows with the tick's sync.
    fn game_insert(&mut self, place: &ItemPlace, stack: ItemStack, tx: &Transaction) -> u32 {
        crate::phase::unprovided(
            "BackendMut::game_insert",
            "game_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemAccess.extract` at those places: up to `stack.count` of the item and components of
    /// `stack`, how many came out. An item entity emptied stays, empty, until the root commit
    /// removes it. A creative player's hand gives without losing any (`forInfiniteMaterials`).
    fn game_extract(&mut self, place: &ItemPlace, stack: ItemStack, tx: &Transaction) -> u32 {
        crate::phase::unprovided(
            "BackendMut::game_extract",
            "game_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Drops `stack` at the middle of the block at `pos`, as a block entity's handler gives up
    /// what it can't put back.
    fn drop_at(&mut self, pos: [i32; 3], stack: ItemStack) {
        crate::phase::unprovided(
            "BackendMut::drop_at",
            "drop_at",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
