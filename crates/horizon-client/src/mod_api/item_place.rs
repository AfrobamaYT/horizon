//! Items as holders (arismoko/horizon#41 §7): an item's own capability, such as
//! [`ITEM_ITEMS`] for a backpack's slots, reached where its stack lives. NeoForge 26.3 reaches
//! an item through an `ItemAccess`; here the place is an [`ItemPlace`], and every read and write
//! of a place goes through [`place_stack`](dyn Host::place_stack) and
//! [`set_place_stack`](dyn Host::set_place_stack), so the inventory holding the stack is the one
//! changed. Handlers hold no borrow of the game, as block entity capabilities don't (#71).
use super::{Direction, Hand, Host, ITEMS, ItemStack};
use std::{
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

/// Where a stack lives.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemPlace {
    /// Slot `slot` of a player's inventory, by `native_inventory_tick`'s numbers: 0..36 (0..9
    /// the hotbar), the armour 100..=103 (feet to head), the off hand -106
    /// (`ItemAccess.forPlayerSlot`).
    PlayerSlot { player: u128, slot: i32 },
    /// What a player holds in `hand` (`forPlayerInteraction`).
    PlayerHand { player: u128, hand: Hand },
    /// The stack on a player's cursor in a menu (`forPlayerCursor`).
    PlayerCursor { player: u128 },
    /// Slot `slot` of the block entity at `pos` as `side` reaches it, through its [`ITEMS`]
    /// handler (`forHandlerIndex`).
    BlockEntity {
        pos: [i32; 3],
        side: Option<Direction>,
        slot: usize,
    },
    /// The stack of item entity `id` in the level in place.
    ItemEntity { id: u32 },
    /// Slot `slot` of the item at `parent`, through its [`ITEM_ITEMS`] handler: a backpack in a
    /// backpack.
    Nested { parent: Box<ItemPlace>, slot: usize },
    /// A place a mod keeps, such as an accessory slot: read and written by the [`PlaceProvider`]
    /// it registered as `provider`, which alone knows what `key` means.
    Mod { provider: &'static str, key: u64 },
}

/// A mod's own places ([`ItemPlace::Mod`]), registered with
/// [`Registrar::place_provider`](super::Registrar::place_provider). Like a handler, it holds no
/// borrow: its stacks live where the mod keeps them (a player's attachments).
pub trait PlaceProvider: Send + Sync + 'static {
    /// The stack at `key`, if any.
    fn stack(&self, host: &mut dyn Host, key: u64) -> Option<ItemStack>;
    /// Puts `stack` in place of what is at `key` (a count of 0 empties it); false where `key`
    /// is no longer a place.
    fn set_stack(&self, host: &mut dyn Host, key: u64, stack: ItemStack) -> bool;
    /// Where an item split off the stack at `key` goes, as vanilla's bucket gives a filled one
    /// to the player, else drops it: what went nowhere.
    fn rest(&self, host: &mut dyn Host, key: u64, rest: ItemStack) -> Option<ItemStack>;
}

/// An item capability: its name, the handler trait `T` it answers with, and the game's own
/// answer for a stack where no part is registered for its item, if the game has one.
pub struct ItemCapabilityKey<T: ?Sized + 'static> {
    pub name: &'static str,
    default: Option<fn(&mut dyn Host, &ItemStack) -> Option<Arc<T>>>,
    handler: PhantomData<fn() -> Arc<T>>,
}

impl<T: ?Sized + 'static> ItemCapabilityKey<T> {
    /// A mod's own item capability, answered only by the parts registered for it.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            default: None,
            handler: PhantomData,
        }
    }
}

/// Items held by an item, as NeoForge 26.3's `Capabilities.Item.ITEM`. The game answers it for
/// an item with `item_container` slots, kept in its `minecraft:container`.
pub const ITEM_ITEMS: ItemCapabilityKey<dyn StackItemHandler> = ItemCapabilityKey {
    name: "horizon:item_items",
    default: Some(|host, stack| {
        static GAME: LazyLock<Arc<dyn StackItemHandler>> =
            LazyLock::new(|| Arc::new(GameItemItems));
        host.item_container_slots(stack).map(|_| GAME.clone())
    }),
    handler: PhantomData,
};

/// The slots of the item at `at`, numbered from 0. A change rewrites the item at `at` through
/// [`set_place_stack`](dyn Host::set_place_stack) with `one_by_one`: from a stack of more than
/// one, the changed item is split off. `simulate` changes nothing.
pub trait StackItemHandler: Send + Sync + 'static {
    fn slots(&self, host: &mut dyn Host, at: &ItemPlace) -> usize;
    fn stack(&self, host: &mut dyn Host, at: &ItemPlace, slot: usize) -> Option<ItemStack>;
    /// Puts as much of `stack` as slot `slot` takes; what didn't fit.
    fn insert(
        &self,
        host: &mut dyn Host,
        at: &ItemPlace,
        slot: usize,
        stack: ItemStack,
        simulate: bool,
    ) -> Option<ItemStack>;
    /// Takes up to `max` out of slot `slot`; what came out.
    fn extract(
        &self,
        host: &mut dyn Host,
        at: &ItemPlace,
        slot: usize,
        max: u32,
        simulate: bool,
    ) -> Option<ItemStack>;
}

/// The game's own [`ITEM_ITEMS`], over the `item_container` helpers. An insert goes where
/// `SimpleContainer.addItem` puts it (equal stacks first, then empty slots), whatever `slot`.
struct GameItemItems;
impl StackItemHandler for GameItemItems {
    fn slots(&self, host: &mut dyn Host, at: &ItemPlace) -> usize {
        host.place_stack(at)
            .and_then(|held| host.item_container_slots(&held))
            .map_or(0, |slots| slots.len())
    }
    fn stack(&self, host: &mut dyn Host, at: &ItemPlace, slot: usize) -> Option<ItemStack> {
        let held = host.place_stack(at)?;
        host.item_container_slots(&held)?.get(slot)?.clone()
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        at: &ItemPlace,
        _: usize,
        stack: ItemStack,
        simulate: bool,
    ) -> Option<ItemStack> {
        let Some((went, changed)) = host
            .place_stack(at)
            .and_then(|held| host.item_container_insert(&held, &stack))
        else {
            return Some(stack);
        };
        if went == 0 || (!simulate && host.set_place_stack(at, changed, true).is_some()) {
            return Some(stack);
        }
        (went < stack.count).then(|| ItemStack {
            count: stack.count - went,
            ..stack
        })
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        at: &ItemPlace,
        slot: usize,
        max: u32,
        simulate: bool,
    ) -> Option<ItemStack> {
        let held = host.place_stack(at)?;
        let (out, changed) = host.item_container_extract(&held, slot, max)?;
        let out = out?;
        if !simulate && host.set_place_stack(at, changed, true).is_some() {
            return None;
        }
        Some(out)
    }
}

impl dyn Host + '_ {
    /// `key`'s handler for the item at `place`: the part a mod registered for its item, else
    /// the game's own answer, if the game has one.
    pub fn item_capability<T: ?Sized + 'static>(
        &mut self,
        key: &ItemCapabilityKey<T>,
        place: &ItemPlace,
    ) -> Option<Arc<T>> {
        let stack = self.place_stack(place)?;
        super::item_capability(key, || Some(&*stack.item))
            .or_else(|| key.default.and_then(|default| default(self, &stack)))
    }

    /// The stack at `place`, if any: the one place every item handler reads.
    pub fn place_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        match place {
            ItemPlace::BlockEntity { pos, side, slot } => self
                .capability(&ITEMS, *pos)?
                .stack(self, *pos, *side, *slot),
            ItemPlace::Nested { parent, slot } => self
                .item_capability(&ITEM_ITEMS, parent)?
                .stack(self, parent, *slot),
            ItemPlace::Mod { provider, key } => super::place_provider(provider)?.stack(self, *key),
            _ => self.game_stack(place),
        }
        .filter(|stack| stack.count > 0)
    }

    /// Puts `stack` at `place` (a count of 0 empties it): the one place every item handler
    /// writes. With `one_by_one`, `stack` stands for one item of what is there: from a stack of
    /// more than one, that one is taken off and `stack` goes where vanilla's
    /// `ItemUtils.createFilledResult` puts a filled bucket, the player's inventory, else dropped
    /// (by the block or item entity, or by the mod's [`PlaceProvider::rest`]).
    ///
    /// None when it's done. Otherwise the stack that went nowhere is the caller's: all of
    /// `stack` when `place` couldn't be written and nothing changed.
    pub fn set_place_stack(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        one_by_one: bool,
    ) -> Option<ItemStack> {
        if one_by_one
            && let Some(held) = self.place_stack(place)
            && held.count > 1
        {
            let kept = ItemStack {
                count: held.count - 1,
                ..held
            };
            if !self.write_place(place, kept) {
                return Some(stack);
            }
            return self.place_rest(place, stack);
        }
        (!self.write_place(place, stack.clone())).then_some(stack)
    }

    /// Replaces the stack at `place`. Through a handler, the old stack is taken out and the new
    /// one put in; what the slot won't take goes to `place_rest`.
    fn write_place(&mut self, place: &ItemPlace, stack: ItemStack) -> bool {
        match place {
            ItemPlace::BlockEntity { pos, side, slot } => {
                let Some(handler) = self.capability(&ITEMS, *pos) else {
                    return false;
                };
                if let Some(old) = handler.stack(self, *pos, *side, *slot)
                    && handler
                        .extract(self, *pos, *side, *slot, old.count, false)
                        .is_none()
                {
                    return false;
                }
                if stack.count > 0
                    && let Some(rest) = handler.insert(self, *pos, *side, *slot, stack, false)
                {
                    self.place_rest(place, rest);
                }
                true
            }
            ItemPlace::Nested { parent, slot } => {
                let Some(handler) = self.item_capability(&ITEM_ITEMS, parent) else {
                    return false;
                };
                if let Some(old) = handler.stack(self, parent, *slot)
                    && handler
                        .extract(self, parent, *slot, old.count, false)
                        .is_none()
                {
                    return false;
                }
                if stack.count > 0
                    && let Some(rest) = handler.insert(self, parent, *slot, stack, false)
                {
                    self.place_rest(place, rest);
                }
                true
            }
            ItemPlace::Mod { provider, key } => super::place_provider(provider)
                .is_some_and(|provider| provider.set_stack(self, *key, stack)),
            _ => self.set_game_stack(place, stack),
        }
    }

    /// Where an item that leaves `place` goes: into the container holding it, else as the game
    /// or the mod puts it. What went nowhere.
    fn place_rest(&mut self, place: &ItemPlace, rest: ItemStack) -> Option<ItemStack> {
        match place {
            ItemPlace::BlockEntity { pos, side, .. } => {
                let mut rest = Some(rest);
                if let Some(handler) = self.capability(&ITEMS, *pos) {
                    for slot in 0..handler.slots(self, *pos, *side) {
                        let Some(stack) = rest.take() else { break };
                        rest = handler.insert(self, *pos, *side, slot, stack, false);
                    }
                }
                rest.and_then(|rest| self.game_rest(place, rest))
            }
            ItemPlace::Nested { parent, .. } => {
                let mut rest = Some(rest);
                if let Some(handler) = self.item_capability(&ITEM_ITEMS, parent) {
                    for slot in 0..handler.slots(self, parent) {
                        let Some(stack) = rest.take() else { break };
                        rest = handler.insert(self, parent, slot, stack, false);
                    }
                }
                rest.and_then(|rest| self.place_rest(parent, rest))
            }
            ItemPlace::Mod { provider, key } => match super::place_provider(provider) {
                Some(provider) => provider.rest(self, *key, rest),
                None => Some(rest),
            },
            _ => self.game_rest(place, rest),
        }
    }
}
