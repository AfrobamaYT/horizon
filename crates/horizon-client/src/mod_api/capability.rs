//! Capability parts (arismoko/horizon#41 §7): code a mod registers for its block entity type to
//! answer a capability, such as [`ITEMS`], in place of the game's own answer. A handler holds no
//! borrow of the game: each call is given the host, so a handler may look up and call a
//! neighbour's capability (a pipe), and it keeps its state where the game keeps the block
//! entity's (its slots, its saved compound, its attachments).
use super::{Direction, Host, ItemStack};
use std::{
    cell::OnceCell,
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

/// A capability: its name (`horizon:items`, `mymod:chemicals`), the handler trait `T` it answers
/// with, and the game's own answer where no part is registered, if the game has one.
pub struct CapabilityKey<T: ?Sized + 'static> {
    pub name: &'static str,
    pub(super) default: Option<fn(&dyn Host, [i32; 3]) -> Option<Arc<T>>>,
    pub(super) handler: PhantomData<fn() -> Arc<T>>,
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
    default: Some(|host, pos| {
        static GAME_ITEMS: LazyLock<Arc<dyn ItemHandler>> = LazyLock::new(|| Arc::new(GameItems));
        host.container_size(pos, None).map(|_| GAME_ITEMS.clone())
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
    /// `key`'s handler for the block entity at `pos`: the part a mod registered for its type,
    /// else the game's own answer, if the game has one there.
    pub fn capability<T: ?Sized + 'static>(
        &self,
        key: &CapabilityKey<T>,
        pos: [i32; 3],
    ) -> Option<Arc<T>> {
        let kind = OnceCell::new();
        super::block_entity_capability(key, || {
            kind.get_or_init(|| self.block_entity_type(pos)).as_deref()
        })
        .or_else(|| key.default.and_then(|default| default(self, pos)))
    }

    /// `handler`'s real insert of `stack` into `slot` of the block entity at `at`, through
    /// `side`: what is left, of the offered item and never more than was offered. A handler
    /// answering more, or another item, is a mod bug, warned once naming its mod.
    pub(crate) fn insert_checked(
        &mut self,
        handler: &Arc<dyn ItemHandler>,
        at: [i32; 3],
        side: Option<Direction>,
        slot: usize,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        let rest = handler
            .insert(self, at, side, slot, stack.clone(), false)
            .filter(|rest| rest.count > 0)?;
        if rest.count > stack.count || !rest.same_item_same_components(&stack) {
            super::item_place::warn_broken(super::capability_mod(handler), &at);
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
        handler: &Arc<dyn ItemHandler>,
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
            super::item_place::warn_broken(super::capability_mod(handler), &at);
            if let Some(rest) = self.insert_checked(handler, at, side, slot, out) {
                self.drop_at(at, rest);
            }
            return None;
        }
        if out.count > max {
            super::item_place::warn_broken(super::capability_mod(handler), &at);
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
