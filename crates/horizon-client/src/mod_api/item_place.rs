//! Items as holders (arismoko/horizon#41 §7): an item's own capability, such as
//! [`ITEM_ITEMS`] for a backpack's slots, reached where its stack lives. NeoForge 26.3 reaches
//! an item through an `ItemAccess`; here the place is an [`ItemPlace`], and every read and write
//! of a place goes through [`place_stack`](dyn Host::place_stack) and
//! [`set_place_stack`](dyn Host::set_place_stack), so the inventory holding the stack is the one
//! changed. Handlers hold no borrow of the game, as block entity capabilities don't (#71).
//!
//! A write is all or nothing: it changes the place and puts every item it moves somewhere, or
//! it changes nothing and hands the stack back. No item is made or lost on the way.
use super::{Direction, Hand, Host, ITEMS, ItemHandler, ItemStack};
use std::{
    cell::RefCell,
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
    /// Puts `stack` in place of what is at `key` (a count of 0 empties it). With `one_by_one`,
    /// `stack` stands for one item of what is there: from a stack of more than one, `key` keeps
    /// the others and `stack` goes where the mod puts a split-off item, such as the player's
    /// inventory, else the ground (`host.set_game_stack` does both for a player's place). All
    /// or nothing: false when nothing changed, as where `key` is no longer a place.
    fn set_stack(&self, host: &mut dyn Host, key: u64, stack: ItemStack, one_by_one: bool) -> bool;
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
/// one, the changed item is split off. When that hands the item back, nothing changed, and the
/// handler reports nothing moved. `simulate` changes nothing.
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

/// The game's own [`ITEM_ITEMS`], over the `item_container` helpers.
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
        slot: usize,
        stack: ItemStack,
        simulate: bool,
    ) -> Option<ItemStack> {
        let Some((went, changed)) = host
            .place_stack(at)
            .and_then(|held| host.item_container_insert_slot(&held, slot, &stack))
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

thread_local! {
    /// One item of the stack at a place, held here while a write into one of its slots
    /// (`set_nested`) changes it: reads and writes of that place come here, so nothing real
    /// changes until the whole write is known to go through.
    static STAGED: RefCell<Vec<(ItemPlace, ItemStack)>> = const { RefCell::new(Vec::new()) };
}

/// Drops the newest staged item, also when a handler panics.
struct Unstage;
impl Drop for Unstage {
    fn drop(&mut self) {
        STAGED.with_borrow_mut(Vec::pop);
    }
}

/// The slots a place sits in: a block entity's, through [`ITEMS`], or an item's, through
/// [`ITEM_ITEMS`].
enum Container<'p> {
    Block(Arc<dyn ItemHandler>, [i32; 3], Option<Direction>),
    Item(Arc<dyn StackItemHandler>, &'p ItemPlace),
}

impl Container<'_> {
    fn slots(&self, host: &mut dyn Host) -> usize {
        match self {
            Self::Block(h, pos, side) => h.slots(host, *pos, *side),
            Self::Item(h, at) => h.slots(host, at),
        }
    }
    fn stack(&self, host: &mut dyn Host, slot: usize) -> Option<ItemStack> {
        match self {
            Self::Block(h, pos, side) => h.stack(host, *pos, *side, slot),
            Self::Item(h, at) => h.stack(host, at, slot),
        }
    }
    fn insert(&self, host: &mut dyn Host, slot: usize, stack: ItemStack) -> Option<ItemStack> {
        match self {
            Self::Block(h, pos, side) => h.insert(host, *pos, *side, slot, stack, false),
            Self::Item(h, at) => h.insert(host, at, slot, stack, false),
        }
    }
    fn extract(&self, host: &mut dyn Host, slot: usize, max: u32) -> Option<ItemStack> {
        match self {
            Self::Block(h, pos, side) => h.extract(host, *pos, *side, slot, max, false),
            Self::Item(h, at) => h.extract(host, at, slot, max, false),
        }
    }
    /// Takes exactly `count` out of `slot`; when less comes out, puts it back and is false.
    fn take(&self, host: &mut dyn Host, slot: usize, count: u32) -> bool {
        match self.extract(host, slot, count) {
            Some(out) if out.count == count => true,
            Some(out) => {
                self.put_back(host, slot, out);
                false
            }
            None => false,
        }
    }
    /// Puts what just came out of `slot` back in. A handler that won't take back what it just
    /// gave breaks its contract, and is named.
    fn put_back(&self, host: &mut dyn Host, slot: usize, stack: ItemStack) {
        if let Some(rest) = self.insert(host, slot, stack) {
            log::error!("Horizon: an item handler refused {rest:?} back into slot {slot}");
        }
    }
    /// Puts all of `stack` into the slots, in order; false, with nothing put in, when it
    /// doesn't all fit.
    fn insert_anywhere(&self, host: &mut dyn Host, stack: &ItemStack) -> bool {
        let mut went = Vec::new();
        let mut left = stack.clone();
        for slot in 0..self.slots(host) {
            let count = left.count;
            match self.insert(host, slot, left.clone()) {
                None => return true,
                Some(rest) => {
                    went.extend((rest.count < count).then_some((slot, count - rest.count)));
                    left = rest;
                }
            }
        }
        for (slot, count) in went {
            if !self.take(host, slot, count) {
                log::error!("Horizon: an item handler kept {count} of {stack:?} in slot {slot}");
            }
        }
        false
    }
    /// Puts `stack` in `slot` in place of what is there. With `one_by_one`, from a stack of
    /// more than one, one item comes out and `stack` goes into whichever slots take it
    /// (`ItemAccess.forHandlerIndex`). All or nothing.
    fn set(&self, host: &mut dyn Host, slot: usize, stack: ItemStack, one_by_one: bool) -> bool {
        let old = self.stack(host, slot);
        if one_by_one && let Some(old) = old.as_ref().filter(|old| old.count > 1) {
            if !self.take(host, slot, 1) {
                return false;
            }
            if stack.count == 0 || self.insert_anywhere(host, &stack) {
                return true;
            }
            let one = ItemStack {
                count: 1,
                ..old.clone()
            };
            self.put_back(host, slot, one);
            return false;
        }
        if let Some(old) = &old
            && !self.take(host, slot, old.count)
        {
            return false;
        }
        let Some(rest) = (stack.count > 0)
            .then(|| self.insert(host, slot, stack.clone()))
            .flatten()
        else {
            return true;
        };
        let went = stack.count - rest.count.min(stack.count);
        if went > 0 && !self.take(host, slot, went) {
            log::error!("Horizon: an item handler kept {went} of {stack:?} in slot {slot}");
        }
        if let Some(old) = old {
            self.put_back(host, slot, old);
        }
        false
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
        let staged = STAGED.with_borrow(|staged| {
            staged
                .iter()
                .rev()
                .find(|(at, _)| at == place)
                .map(|(_, stack)| stack.clone())
        });
        match (staged, place) {
            (Some(stack), _) => Some(stack),
            (None, ItemPlace::BlockEntity { pos, side, slot }) => self
                .capability(&ITEMS, *pos)?
                .stack(self, *pos, *side, *slot),
            (None, ItemPlace::Nested { parent, slot }) => self
                .item_capability(&ITEM_ITEMS, parent)?
                .stack(self, parent, *slot),
            (None, ItemPlace::Mod { provider, key }) => {
                super::place_provider(provider)?.stack(self, *key)
            }
            (None, _) => self.game_stack(place),
        }
        .filter(|stack| stack.count > 0)
    }

    /// Puts `stack` at `place` (a count of 0 empties it): the one place every item handler
    /// writes. With `one_by_one`, `stack` stands for one item of what is there, as
    /// `ItemAccess.oneByOne`: from a stack of more than one, the place keeps the others and
    /// `stack` goes elsewhere. From a player's hand it goes as vanilla's
    /// `ItemUtils.createFilledResult` puts a filled bucket: into their inventory, else dropped,
    /// and in creative the hand is kept. From a player's other places the same, without
    /// creative's rule. From an item entity, it's dropped there. From a block entity's slot or
    /// an item's, into that container's other slots. From a mod's place, where its
    /// [`PlaceProvider`] puts it. A stack of items holding the place splits the same way: one
    /// of them is rewritten holding the change, and the others stay as they were.
    ///
    /// All or nothing: None when it's done, Some(`stack`) when it couldn't be and nothing
    /// changed.
    pub fn set_place_stack(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        one_by_one: bool,
    ) -> Option<ItemStack> {
        let staged = STAGED.with_borrow_mut(|staged| {
            let at = staged.iter_mut().rev().find(|(at, _)| at == place);
            at.map(|(_, held)| *held = stack.clone()).is_some()
        });
        let done = staged
            || match place {
                ItemPlace::BlockEntity { pos, side, slot } => {
                    self.capability(&ITEMS, *pos).is_some_and(|handler| {
                        Container::Block(handler, *pos, *side).set(
                            self,
                            *slot,
                            stack.clone(),
                            one_by_one,
                        )
                    })
                }
                ItemPlace::Nested { parent, slot } => {
                    self.set_nested(parent, *slot, stack.clone(), one_by_one)
                }
                ItemPlace::Mod { provider, key } => {
                    super::place_provider(provider).is_some_and(|provider| {
                        provider.set_stack(self, *key, stack.clone(), one_by_one)
                    })
                }
                _ => self.set_game_stack(place, stack.clone(), one_by_one),
            };
        (!done).then_some(stack)
    }

    /// Writes slot `slot` of the item at `parent` on one item of it, staged, then writes that
    /// one item back to `parent` with `one_by_one`: from a stack of them, it splits off holding
    /// the change. Nothing real changes before that last write, so a refusal anywhere changes
    /// nothing.
    fn set_nested(
        &mut self,
        parent: &ItemPlace,
        slot: usize,
        stack: ItemStack,
        one_by_one: bool,
    ) -> bool {
        let Some(handler) = self.item_capability(&ITEM_ITEMS, parent) else {
            return false;
        };
        let Some(before) = self.place_stack(parent) else {
            return false;
        };
        let one = ItemStack { count: 1, ..before };
        STAGED.with_borrow_mut(|staged| staged.push((parent.clone(), one.clone())));
        let unstage = Unstage;
        let done = Container::Item(handler, parent).set(self, slot, stack, one_by_one);
        let after = STAGED.with_borrow(|staged| staged.last().map(|(_, held)| held.clone()));
        drop(unstage);
        match after {
            Some(after) if done => {
                after == one || self.set_place_stack(parent, after, true).is_none()
            }
            _ => false,
        }
    }
}
