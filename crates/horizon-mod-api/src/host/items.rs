//! The `items` area: host methods the game implements (`game/host/items.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `items` area, implemented by the game and reached through [`Host`].
pub trait ItemsHost {
    /// Side: both.
    /// The most of the item `item` one slot holds (`Item.getDefaultMaxStackSize`),
    /// or `None` for an unknown item: for a mod's container that fills or
    /// merges stacks itself.
    fn max_stack_size(&self, item: &str) -> Option<u32> {
        crate::phase::unprovided(
            "Host::max_stack_size",
            "max_stack_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Item `item`'s default value of component `component`, in its prototype
    /// (`Item.components().get`): `minecraft:max_damage` on an iron sword is `250`, data
    /// packs' and native mods' changes included. Read from the item report's JSON, so a
    /// number is an `Int`, `Long` or `Double` and a boolean a `Byte`. None where the item
    /// hasn't the component, and for an unknown item.
    fn item_default_component(&self, item: &str, component: &str) -> Option<Tag> {
        crate::phase::unprovided(
            "Host::item_default_component",
            "item_default_component",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Item `item`'s cooking-fuel burn time in ticks (`FuelValues.burnDuration` of its `cooking_fuel`
    /// component, the data pack's context int provider resolved), None where it is no fuel. API
    /// @API:item_components@.
    fn item_burn_time(&self, item: &str) -> Option<i32> {
        crate::phase::unprovided(
            "Host::item_burn_time",
            "item_burn_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Item `item`'s chance to raise a composter's level (its `compostable` component's layers provider
    /// resolved), None where it is not compostable. API @API:item_components@.
    fn item_compost_chance(&self, item: &str) -> Option<f32> {
        crate::phase::unprovided(
            "Host::item_compost_chance",
            "item_compost_chance",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether enchantments `a` and `b` may share an item (`Enchantment.areCompatible`), by the
    /// world's enchantment definitions. API @API:item_components@.
    fn enchantments_compatible(&self, a: &str, b: &str) -> bool {
        crate::phase::unprovided(
            "Host::enchantments_compatible",
            "enchantments_compatible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
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
/// The server host's methods of the `items` area, implemented by the game and reached through [`ServerHost`].
pub trait ItemsServerHost {
    /// Side: server.
    /// Drop one item as DefaultDispenseItemBehavior does. For a dispenser hook:
    /// return this remainder in its answer with `CLICK`; the caller plays the
    /// answer's event and smoke once. A client, another dimension or an unloaded
    /// chunk keeps the input stack unchanged.
    fn dispense_default(
        &mut self,
        dimension: &str,
        pos: [i32; 3],
        facing: Direction,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "ServerHost::dispense_default",
            "dispense_default",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `items` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait ItemsClientHost {
    /// Side: client.
    /// Creative mode only: gives the player `count` of `item`
    /// (`minecraft:diamond`) as taking it from the creative inventory does
    /// (`ServerboundSetCreativeModeSlot`): the selected slot first, then the
    /// hotbar, then the inventory. The count is cut to one stack.
    fn give(&mut self, item: &str, count: u32) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::give",
            "give",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Registered enchantments, in registry order.
    fn enchantments(&self) -> Vec<EnchantmentInfo> {
        crate::phase::unprovided(
            "ClientHost::enchantments",
            "enchantments",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// `Enchantment.getMinCost(level)`: the enchanting power needed, or None for an unknown id.
    fn enchantment_min_cost(&self, id: &str, level: u32) -> Option<i32> {
        crate::phase::unprovided(
            "ClientHost::enchantment_min_cost",
            "enchantment_min_cost",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Whether enchantment `id` can go on `item` (`Enchantment.isSupportedItem`).
    fn enchantment_supports(&self, id: &str, item: &str) -> bool {
        crate::phase::unprovided(
            "ClientHost::enchantment_supports",
            "enchantment_supports",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
/// The handles' reads of the `items` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemsBackend {
    /// Side: both. ItemResource's effective use duration, including its Part.
    fn item_resource_use_duration(&self, stack: &ItemStack) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::item_resource_use_duration",
            "item_resource_use_duration",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both. The effective vanilla bar, before client drawing hooks.
    fn item_resource_bar(&self, stack: &ItemStack) -> Option<ItemBar> {
        crate::phase::unprovided(
            "Backend::item_resource_bar",
            "item_resource_bar",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many slots of the block entity at `at` its side reaches (none: every slot), by its
    /// `WorldlyContainer` rules: a furnace's top reaches its input only. None where no block
    /// entity has slots, in a level not in place, and on a client's copy of the world.
    fn container_size(&self, at: BlockSide) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::container_size",
            "container_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The stack in slot `slot` of those `at`'s side reaches (`native_container_size`).
    fn container_stack(&self, at: BlockSide, slot: usize) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::container_stack",
            "container_stack",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many of `stack` slot `slot` of those `at`'s side reaches holds when empty, as
    /// NeoForge's `isValid` and `getCapacityAsLong` answer: none where it may not go in (a
    /// furnace's output, `canPlaceItem`), else the slot's limit for it.
    fn container_capacity(&self, at: BlockSide, slot: usize, stack: &ItemStack) -> u32 {
        crate::phase::unprovided(
            "Backend::container_capacity",
            "container_capacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether an enchantment on `stack` has effect component `effect`, by the world's
    /// enchantment definitions (`EnchantmentHelper.has`), behind
    /// `ItemResource::has_enchantment_effect`, through the opaque backend. API
    /// @API:item_components@.
    fn item_has_enchantment_effect(&self, stack: &ItemStack, effect: &str) -> bool {
        crate::phase::unprovided(
            "Backend::item_has_enchantment_effect",
            "item_has_enchantment_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The slot `stack` is equipped to (`Equippable.slot`) after its component patch, behind
    /// `ItemResource::equippable_slot`, through the opaque backend. API @API:item_components@.
    fn item_equippable_slot(&self, stack: &ItemStack) -> Option<String> {
        crate::phase::unprovided(
            "Backend::item_equippable_slot",
            "item_equippable_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Capabilities.Item.ITEM`, read: the slots of one item of `stack`,
    /// empty ones included, kept in its `minecraft:container`. None for an
    /// item without slots, and for an empty stack.
    fn item_container_slots(&self, stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
        crate::phase::unprovided(
            "Backend::item_container_slots",
            "item_container_slots",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ResourceHandler.insert`: as much of `item` as fits into one item of
    /// `stack`, topping up the slots that hold the same item and components
    /// first, then the first empty ones (`SimpleContainer.addItem`'s order),
    /// no slot past its limit (99, or the item's own when less). A shulker
    /// box never goes in. The count that went in, and the single item it
    /// becomes (the item itself when none did). The stack given is not
    /// changed: shrink it by one and hand out the result, or leave the result
    /// unused to simulate. None where the item has no slots.
    fn item_container_insert(
        &self,
        stack: &ItemStack,
        item: &ItemStack,
    ) -> Option<(u32, ItemStack)> {
        crate::phase::unprovided(
            "Backend::item_container_insert",
            "item_container_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ResourceHandler.extract`: up to `max` out of slot `slot` of one item
    /// of `stack`. What came out (none when the slot is empty or past the
    /// item's slots), and the single item it becomes. The stack given is not
    /// changed, as with `native_item_container_insert`. None where the item
    /// has no slots.
    fn item_container_extract(
        &self,
        stack: &ItemStack,
        slot: usize,
        max: u32,
    ) -> Option<(Option<ItemStack>, ItemStack)> {
        crate::phase::unprovided(
            "Backend::item_container_extract",
            "item_container_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
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
}
/// The handles' writes of the `items` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemsBackendMut {
    /// Side: server.
    /// Puts as much of `stack` as slot `slot` of those `at`'s side reaches takes, as a hopper
    /// would (`canPlaceItem`, the slot's limit, the same item and components), in transaction
    /// `tx`; what didn't fit.
    fn container_insert(
        &mut self,
        at: BlockSide,
        slot: usize,
        stack: ItemStack,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::container_insert",
            "container_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Takes up to `max` out of slot `slot` of those `at`'s side reaches, in transaction `tx`;
    /// through a side, only what that side may take (`canTakeItemThroughFace`). What came out.
    fn container_extract(
        &mut self,
        at: BlockSide,
        slot: usize,
        max: u32,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::container_extract",
            "container_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
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
