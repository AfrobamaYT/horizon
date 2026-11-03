//! Capability parts (arismoko/horizon#41 §7): providers a mod registers for its block entity
//! type to answer a capability, such as [`ITEMS`], beside the game's own (NeoForge's
//! `RegisterCapabilitiesEvent`). A handler holds no borrow of the game: each call is given the
//! host, so a handler may look up and call a neighbour's capability (a pipe), and it keeps its
//! state where the game keeps the block entity's (its slots, its saved compound, its
//! attachments).
use super::{Direction, Host, ItemStack};
use std::{marker::PhantomData, ops::Deref};

/// A capability: its name (`horizon:items`, `mymod:chemicals`), the handler trait `T` it answers
/// with, and the game's own provider, if the game has one.
pub struct CapabilityKey<T: ?Sized + 'static> {
    pub name: &'static str,
    pub(super) default: Option<fn(&mut dyn Host, BlockSide) -> Option<Box<T>>>,
    pub(super) handler: PhantomData<fn() -> Box<T>>,
}

/// A block entity a capability is asked of, and the side it is reached through (none: from no
/// particular side): NeoForge's block capability context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockSide {
    pub pos: [i32; 3],
    pub side: Option<Direction>,
}

/// A capability's handler, with who answered: a mod's provider or the game's own.
pub struct Capability<T: ?Sized> {
    pub(super) handler: Box<T>,
    /// The mod whose provider answered; none for the game.
    pub(super) owner: Option<usize>,
}

impl<T: ?Sized> Deref for Capability<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.handler
    }
}

impl<T: ?Sized> Capability<T> {
    /// The id of the mod whose provider answered, or "the game", for warnings.
    pub fn owner(&self) -> String {
        self.owner
            .map_or_else(|| "the game".to_owned(), super::mod_name)
    }
}

impl<T: ?Sized + 'static> CapabilityKey<T> {
    /// A mod's own capability, answered only by the parts registered for it.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            default: None,
            handler: PhantomData,
        }
    }
}

/// Item slots, as NeoForge 26.3's `Capabilities.Item.BLOCK` (a `ResourceHandler<ItemResource>`).
/// The game answers it for every block entity with slots: chests (a double chest's two halves as
/// one), furnaces, hoppers and the other vanilla containers, by their `WorldlyContainer` sides,
/// and a mod block entity's `block_entity_container`.
pub const ITEMS: CapabilityKey<dyn ItemHandler> = CapabilityKey {
    name: "horizon:items",
    default: Some(|host, at| {
        host.container_size(at.pos, None)
            .map(|_| Box::new(GameItems) as Box<dyn ItemHandler>)
    }),
    handler: PhantomData,
};

/// The item slots of the block entity at `at` as its side `side` reaches them (none: from no
/// particular side, every slot), numbered from 0 for that side. `simulate` changes nothing and
/// must answer exactly as the real call would, as NeoForge's handlers must: a write plans with
/// it, then commits. A handler whose real call disagrees with its own simulation is a mod bug;
/// the game then puts back what it can, drops the rest at the block and warns once. Moves that
/// must succeed together across more handlers need transactions, which are not here yet
/// (#41 §7).
pub trait ItemHandler: Send + Sync + 'static {
    /// How many slots `side` reaches.
    fn slots(&self, host: &dyn Host, at: [i32; 3], side: Option<Direction>) -> usize;
    /// The stack in slot `slot`, if any.
    fn stack(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
    ) -> Option<ItemStack>;
    /// Puts as much of `stack` as slot `slot` takes; what didn't fit.
    fn insert(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: ItemStack,
        simulate: bool,
    ) -> Option<ItemStack>;
    /// Takes up to `max` out of slot `slot`; what came out.
    fn extract(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        max: u32,
        simulate: bool,
    ) -> Option<ItemStack>;
    /// How many of `stack` slot `slot` holds when empty, whatever it holds now: none where
    /// `stack` may not go in, such as an output slot. NeoForge's `isValid` and
    /// `getCapacityAsLong` in one; a write that replaces a slot's stack asks it before taking
    /// anything out.
    fn capacity(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: &ItemStack,
    ) -> u32;
}

/// The game's own [`ITEMS`]: the block entity's slots, by the rules hoppers follow.
struct GameItems;
impl ItemHandler for GameItems {
    fn slots(&self, host: &dyn Host, at: [i32; 3], side: Option<Direction>) -> usize {
        host.container_size(at, side).unwrap_or(0)
    }
    fn stack(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
    ) -> Option<ItemStack> {
        host.container_stack(at, side, slot)
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: ItemStack,
        simulate: bool,
    ) -> Option<ItemStack> {
        host.container_insert(at, side, slot, stack, simulate)
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        max: u32,
        simulate: bool,
    ) -> Option<ItemStack> {
        host.container_extract(at, side, slot, max, simulate)
    }
    fn capacity(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: &ItemStack,
    ) -> u32 {
        host.container_capacity(at, side, slot, stack)
    }
}

impl dyn Host + '_ {
    /// `key`'s handler for the block entity at `pos`, reached through `side`: the first provider
    /// to answer, of those mods registered for its type `before_game`, the game's own, and the
    /// mods' others, each in registration order.
    pub fn capability<T: ?Sized + 'static>(
        &mut self,
        key: &CapabilityKey<T>,
        pos: [i32; 3],
        side: Option<Direction>,
    ) -> Option<Capability<T>> {
        let at = BlockSide { pos, side };
        let mut game = None;
        let answer = super::ask_providers::<super::BlockProvider<T>, T>(
            false,
            key.name,
            self,
            |host| host.block_entity_type(pos),
            |provider, host| provider(host, at),
            |host| {
                game = key.default.and_then(|default| default(host, at));
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

    /// `handler`'s real insert of `stack` into `slot` of the block entity at `at`, through
    /// `side`: what is left, of the offered item and never more than was offered. A handler
    /// answering more, or another item, is a mod bug, warned once naming its mod.
    pub(crate) fn insert_checked(
        &mut self,
        handler: &Capability<dyn ItemHandler>,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        let rest = handler
            .insert(self, at, side, slot, stack.clone(), false)
            .filter(|rest| rest.count > 0)?;
        if rest.count > stack.count || !rest.same_item_same_components(&stack) {
            super::item_place::warn_broken(handler.owner, &at);
        }
        Some(ItemStack {
            count: rest.count.min(stack.count),
            ..stack
        })
    }

    /// `handler`'s real extract from `slot` of the block entity at `at`, through `side`, of what
    /// its simulation gave, `simulated`: what came out, of that item and never more. Another item,
    /// or the same with other components, is a mod bug, warned once naming its mod: it goes back
    /// into the slot, else is dropped at the block, and nothing comes out. So is what it gives
    /// beyond `simulated`'s count, which goes back the same way.
    pub(crate) fn extract_checked(
        &mut self,
        handler: &Capability<dyn ItemHandler>,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        simulated: &ItemStack,
    ) -> Option<ItemStack> {
        let max = simulated.count;
        let out = handler
            .extract(self, at, side, slot, max, false)
            .filter(|out| out.count > 0)?;
        if !out.same_item_same_components(simulated) {
            super::item_place::warn_broken(handler.owner, &at);
            if let Some(rest) = self.insert_checked(handler, at, side, slot, out) {
                self.drop_at(at, rest);
            }
            return None;
        }
        if out.count > max {
            super::item_place::warn_broken(handler.owner, &at);
            let excess = ItemStack {
                count: out.count - max,
                ..out.clone()
            };
            if let Some(rest) = self.insert_checked(handler, at, side, slot, excess) {
                self.drop_at(at, rest);
            }
        }
        (max > 0).then(|| ItemStack {
            count: out.count.min(max),
            ..out
        })
    }
}
