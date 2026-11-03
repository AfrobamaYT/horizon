//! Items as holders (arismoko/horizon#41 §7): an item's own capability, such as
//! [`ITEM_ITEMS`] for a backpack's slots, reached where its stack lives. NeoForge 26.3 reaches
//! an item through an `ItemAccess`; here the place is an [`ItemPlace`], and every read and write
//! of a place goes through [`place_stack`](dyn Host::place_stack) and
//! [`set_place_stack`](dyn Host::set_place_stack), so the inventory holding the stack is the one
//! changed. Handlers hold no borrow of the game, as block entity capabilities don't (#71).
//!
//! A write runs in a transaction of its own, nested in the caller's: the old stack must come out
//! whole and the new one go in whole, or it aborts, and nothing moved. A handler answering more
//! than it was asked is a mod bug, named once, and the write aborts the same way.
use super::{
    Capability, Direction, Hand, Host, ITEMS, ItemResource, ItemStack, ResourceHandler, Transaction,
};
use std::{cell::RefCell, collections::HashSet, fmt::Debug, marker::PhantomData, sync::Mutex};

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
    /// Slot `slot` of the inventory of entity `entity` in the level in place: a chest or hopper
    /// minecart's, a chest boat's, or a chested donkey's, mule's or llama's
    /// (`Capabilities.Item.ENTITY` through `forHandlerIndex`).
    EntitySlot { entity: u32, slot: usize },
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
    /// Puts `stack` in place of what is at `key` (a count of 0 empties it), in transaction `tx`.
    /// With `one_by_one`, `stack` stands for one item of what is there: from a stack of more
    /// than one, `key` keeps the others and `stack` goes where the mod puts a split-off item,
    /// such as the player's inventory, else the ground (`host.set_game_stack` does both for a
    /// player's place). All or nothing: false when nothing changed, as where `key` is no longer
    /// a place.
    fn set_stack(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        key: u64,
        stack: ItemStack,
        one_by_one: bool,
    ) -> bool;
}

/// An item capability: its name, the handler trait `T` it answers with, and the game's own
/// provider, if the game has one.
pub struct ItemCapabilityKey<T: ?Sized + 'static> {
    pub name: &'static str,
    default: Option<fn(&mut dyn Host, &ItemPlace, &ItemStack) -> Option<Box<T>>>,
    handler: PhantomData<fn() -> Box<T>>,
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

/// Items held by an item, as NeoForge 26.3's `Capabilities.Item.ITEM`: a handler for the item at
/// one place, whose changes rewrite the item there through
/// [`set_place_stack`](dyn Host::set_place_stack) with `one_by_one`, so from a stack of more
/// than one the changed item splits off. The game answers it for an item with `item_container`
/// slots, kept in its `minecraft:container`.
pub const ITEM_ITEMS: ItemCapabilityKey<dyn ResourceHandler<ItemResource>> = ItemCapabilityKey {
    name: "horizon:item_items",
    default: Some(|host, place, stack| {
        host.item_container_slots(stack).map(|_| {
            Box::new(GameItemItems(place.clone())) as Box<dyn ResourceHandler<ItemResource>>
        })
    }),
    handler: PhantomData,
};

/// The game's own [`ITEM_ITEMS`] for the item at `.0`, over the `item_container` helpers.
struct GameItemItems(ItemPlace);
impl GameItemItems {
    fn slot(&self, host: &mut dyn Host, index: usize) -> Option<ItemStack> {
        let held = host.place_stack(&self.0)?;
        host.item_container_slots(&held)?.get(index)?.clone()
    }
}
impl ResourceHandler<ItemResource> for GameItemItems {
    fn size(&self, host: &mut dyn Host) -> usize {
        host.place_stack(&self.0)
            .and_then(|held| host.item_container_slots(&held))
            .map_or(0, |slots| slots.len())
    }
    fn resource(&self, host: &mut dyn Host, index: usize) -> Option<ItemResource> {
        Some(ItemResource::of(&self.slot(host, index)?))
    }
    fn amount(&self, host: &mut dyn Host, index: usize) -> u32 {
        self.slot(host, index).map_or(0, |stack| stack.count)
    }
    fn capacity(&self, host: &mut dyn Host, index: usize, resource: &ItemResource) -> u32 {
        host.place_stack(&self.0).map_or(0, |held| {
            host.item_container_capacity(&held, index, &resource.to_stack(1))
        })
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        let offer = resource.to_stack(amount);
        let Some((went, changed)) = host
            .place_stack(&self.0)
            .and_then(|held| host.item_container_insert_slot(&held, index, &offer))
        else {
            return 0;
        };
        match went > 0 && host.set_place_stack(&self.0, changed, true, tx).is_none() {
            true => went.min(amount),
            false => 0,
        }
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        if !self
            .slot(host, index)
            .is_some_and(|held| resource.matches(&held))
        {
            return 0;
        }
        let Some((Some(out), changed)) = host
            .place_stack(&self.0)
            .and_then(|held| host.item_container_extract(&held, index, amount))
        else {
            return 0;
        };
        match host.set_place_stack(&self.0, changed, true, tx).is_none() {
            true => out.count.min(amount),
            false => 0,
        }
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

/// The owners of item handlers that answered more than they were asked, warned once.
static BROKEN: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// `owner`'s handler at `at` answered more than it was asked: a mod bug, warned once per mod.
pub(crate) fn warn_broken(owner: Option<usize>, at: &dyn Debug) {
    let owner = owner.map_or_else(|| "the game".to_owned(), super::mod_name);
    let mut warned = BROKEN.lock().unwrap_or_else(|e| e.into_inner());
    if warned.get_or_insert_default().insert(owner.clone()) {
        log::warn!(
            "Horizon: {owner}'s handler at {at:?} answered more than it was asked; the move it \
             was part of is undone. A handler moves at most what it is asked."
        );
    }
}

#[cfg(test)]
pub(super) fn warned_about(owner: &str) -> bool {
    let broken = BROKEN.lock().unwrap_or_else(|e| e.into_inner());
    broken.as_ref().is_some_and(|owners| owners.contains(owner))
}

/// Puts `stack` in slot `slot` of `handler`, at `at`, in place of what is there, in a transaction
/// of its own: all of the old stack must come out, and all of `stack` go in, in passes until one
/// takes nothing (`Inventory.add` and NeoForge's insert loops: a handler may take less per call
/// than it holds). With `one_by_one`, from a stack of more than one, one item comes out and
/// `stack` goes into the other slots (`ItemAccess.forHandlerIndex`). Anything short aborts it,
/// so nothing moved: false.
fn set_slot(
    handler: &Capability<dyn ResourceHandler<ItemResource>>,
    host: &mut dyn Host,
    at: &dyn Debug,
    slot: usize,
    stack: ItemStack,
    one_by_one: bool,
) -> bool {
    let fill = |host: &mut dyn Host, tx: &Transaction, slot: usize, left: &mut u32| {
        let resource = ItemResource::of(&stack);
        while *left > 0 {
            let asked = *left;
            let went = handler.insert(host, tx, slot, &resource, asked);
            match handler.checked(went, asked, at) {
                Some(0) => break,
                Some(went) => *left -= went,
                None => return Err(()),
            }
        }
        Ok(())
    };
    let result = host.transaction(|host, tx| {
        let take = |host: &mut dyn Host, resource: &ItemResource, asked: u32| {
            let came = handler.extract(host, tx, slot, resource, asked);
            handler
                .checked(came, asked, at)
                .filter(|&came| came == asked)
                .ok_or(())
        };
        let old = handler.resource(host, slot);
        let count = handler.amount(host, slot);
        let mut left = stack.count;
        match old {
            Some(old) if one_by_one && count > 1 => {
                take(host, &old, 1)?;
                for other in (0..handler.size(host)).filter(|&other| other != slot) {
                    fill(host, tx, other, &mut left)?;
                }
            }
            Some(old) => {
                take(host, &old, count)?;
                fill(host, tx, slot, &mut left)?;
            }
            None => fill(host, tx, slot, &mut left)?,
        }
        match left {
            0 => Ok(()),
            _ => Err(()),
        }
    });
    result.is_ok()
}

impl dyn Host + '_ {
    /// `key`'s handler for the item at `place`: the first provider to answer, of those mods
    /// registered for its item `before_game`, the game's own, and the mods' others, each in
    /// registration order.
    pub fn item_capability<T: ?Sized + 'static>(
        &mut self,
        key: &ItemCapabilityKey<T>,
        place: &ItemPlace,
    ) -> Option<Capability<T>> {
        let stack = self.place_stack(place)?;
        let mut game = None;
        let parts = super::providers(true, key.name, self, |_| Some(stack.item.to_string()));
        let answer = super::ask_providers::<super::ItemProvider<T>, T>(
            parts,
            self,
            |provider, host| provider(host, place),
            |host| {
                game = key.default.and_then(|default| default(host, place, &stack));
                game.is_some()
            },
        );
        match answer {
            super::Answer::Mod(found) => Some(found),
            super::Answer::Game => game.map(|handler| Capability {
                handler,
                owner: None,
            }),
            super::Answer::Nobody => None,
        }
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
            (None, ItemPlace::BlockEntity { pos, side, slot }) => {
                let handler = self.capability(&ITEMS, *pos, *side)?;
                let resource = handler.resource(self, *slot)?;
                Some(resource.to_stack(handler.amount(self, *slot)))
            }
            (None, ItemPlace::Nested { parent, slot }) => {
                let handler = self.item_capability(&ITEM_ITEMS, parent)?;
                let resource = handler.resource(self, *slot)?;
                Some(resource.to_stack(handler.amount(self, *slot)))
            }
            (None, ItemPlace::Mod { provider, key }) => {
                super::place_provider(provider)?.stack(self, *key)
            }
            (None, _) => self.game_stack(place),
        }
        .filter(|stack| stack.count > 0)
    }

    /// Puts `stack` at `place` (a count of 0 empties it), in transaction `tx`: the one place
    /// every item handler writes. With `one_by_one`, `stack` stands for one item of what is
    /// there, as `ItemAccess.oneByOne`: from a stack of more than one, the place keeps the
    /// others and `stack` goes elsewhere. From a player's hand it goes as vanilla's
    /// `ItemUtils.createFilledResult` puts a filled bucket: into their inventory, else dropped
    /// at the root commit, and in creative the hand is kept. From a player's other places the
    /// same, without creative's rule. From an item entity, it's dropped there. From a block
    /// entity's slot or an item's, into that container's other slots. From a mod's place, where
    /// its [`PlaceProvider`] puts it. A stack of items holding the place splits the same way:
    /// one of them is rewritten holding the change, and the others stay as they were.
    ///
    /// All or nothing: None when it's done, Some(`stack`) when it can't be, and nothing moved.
    pub fn set_place_stack(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        one_by_one: bool,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        let staged = STAGED.with_borrow_mut(|staged| {
            let at = staged.iter_mut().rev().find(|(at, _)| at == place);
            at.map(|(_, held)| *held = stack.clone()).is_some()
        });
        let done = staged
            || match place {
                ItemPlace::BlockEntity { pos, side, slot } => {
                    self.capability(&ITEMS, *pos, *side).is_some_and(|handler| {
                        set_slot(&handler, self, place, *slot, stack.clone(), one_by_one)
                    })
                }
                ItemPlace::Nested { parent, slot } => {
                    self.set_nested(parent, *slot, stack.clone(), one_by_one)
                }
                ItemPlace::Mod { provider, key } => {
                    super::place_provider(provider).is_some_and(|provider| {
                        provider.set_stack(self, tx, *key, stack.clone(), one_by_one)
                    })
                }
                _ => self.set_game_stack(place, stack.clone(), one_by_one, tx),
            };
        (!done).then_some(stack)
    }

    /// Writes slot `slot` of the item at `parent` on one item of it, staged, then writes that
    /// one item back to `parent` with `one_by_one`: from a stack of them, it splits off holding
    /// the change. All in a transaction of its own, so a refusal anywhere, that last write's
    /// included, undoes what the slot's handler wrote elsewhere.
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
        let place = ItemPlace::Nested {
            parent: Box::new(parent.clone()),
            slot,
        };
        let result = self.transaction(|host, tx| {
            STAGED.with_borrow_mut(|staged| staged.push((parent.clone(), one.clone())));
            let unstage = Unstage;
            let done = set_slot(&handler, host, &place, slot, stack, one_by_one);
            let after = STAGED.with_borrow(|staged| staged.last().map(|(_, held)| held.clone()));
            drop(unstage);
            match after {
                Some(after) if done => {
                    let written =
                        after == one || host.set_place_stack(parent, after, true, tx).is_none();
                    written.then_some(()).ok_or(())
                }
                _ => Err(()),
            }
        });
        result.is_ok()
    }
}
