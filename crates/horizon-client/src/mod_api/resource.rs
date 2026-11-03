//! NeoForge 26.3's transfer API (#41 §7): a resource is what a handler holds, without an amount
//! ([`ItemResource`]), and a [`ResourceHandler`] is the indexed contents of one holder, written
//! only inside a transaction.
use super::{Capability, Compound, Host, ItemStack, Tag, Transaction};
use horizon_world::nbt::JavaString;
use std::fmt;

/// An item and its components, without a count, as NeoForge's `ItemResource`: what a slot holds.
/// An empty slot holds none.
#[derive(Clone)]
pub struct ItemResource(ItemStack);

impl ItemResource {
    /// What `stack` is, whatever its count.
    pub fn of(stack: &ItemStack) -> Self {
        Self(ItemStack {
            count: 1,
            ..stack.clone()
        })
    }
    /// The item's id (`minecraft:apple`).
    pub fn item(&self) -> &str {
        &self.0.item
    }
    /// `count` of it as a stack.
    pub fn to_stack(&self, count: u32) -> ItemStack {
        ItemStack {
            count,
            ..self.0.clone()
        }
    }
    /// Whether `stack` is this item with these components.
    pub fn matches(&self, stack: &ItemStack) -> bool {
        self.0.same_item_same_components(stack)
    }
    /// This item's own value of component `id` (`minecraft:damage`, `minecraft:custom_data`), as
    /// saved (`ItemResource.get`); none where it keeps the item's default.
    pub fn component(&self, id: &str) -> Option<Tag> {
        crate::item_data::encode_components(&self.0)?
            .get(id)
            .cloned()
    }
    /// This item with component `id` set to `value` (`ItemResource.with`). A stack changes where
    /// it lives by an exchange of one of it for this: `place.exchange(host, tx, &changed, 1)`,
    /// which splits it off a bigger stack, in the transaction.
    pub fn with_component(&self, id: &str, value: Tag) -> Self {
        self.patched(id, Some(value))
    }
    /// This item with its own value of component `id` dropped, back to the item's default.
    pub fn without_component(&self, id: &str) -> Self {
        self.patched(id, None)
    }
    /// This item with its own components, as saved, changed at `id`.
    fn patched(&self, id: &str, value: Option<Tag>) -> Self {
        let mut components = match crate::item_data::encode_components(&self.0) {
            Some(Tag::Compound(components)) => components,
            _ => Compound::new(),
        };
        match value {
            Some(value) => components.insert(JavaString::from(id), value),
            None => components.remove(&JavaString::from(id)),
        };
        let (damage, extra) = crate::item_data::decode_components(Some(&Tag::Compound(components)));
        Self(ItemStack {
            item: self.0.item.clone(),
            count: 1,
            damage,
            extra,
        })
    }
}

impl PartialEq for ItemResource {
    fn eq(&self, other: &Self) -> bool {
        self.matches(&other.0)
    }
}
impl Eq for ItemResource {}

impl fmt::Debug for ItemResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ItemResource").field(&self.0.item).finish()
    }
}

/// The contents of one holder (a block entity as one side reaches it, an item where it lives),
/// numbered from 0, as NeoForge's `ResourceHandler`: a provider makes one for its holder, so it
/// knows where its contents live. It holds no borrow of the game: each call is given the host,
/// and its state is the game's (slots, a saved compound, attachments, a stack's components), so
/// the transaction `tx` a write runs in puts it back on an abort. Amounts never exceed what was
/// asked; a handler answering more is a mod bug, and the move it was part of is aborted.
pub trait ResourceHandler<R> {
    /// How many indices it has.
    fn size(&self, host: &mut dyn Host) -> usize;
    /// What index `index` holds, if anything.
    fn resource(&self, host: &mut dyn Host, index: usize) -> Option<R>;
    /// How much of it.
    fn amount(&self, host: &mut dyn Host, index: usize) -> u32;
    /// How much of `resource` index `index` holds when it holds that, whatever it holds now: none
    /// where `resource` may not go (`getCapacityAsLong`).
    fn capacity(&self, host: &mut dyn Host, index: usize, resource: &R) -> u32;
    /// Whether `resource` may go in index `index` at all (`isValid`).
    fn is_valid(&self, host: &mut dyn Host, index: usize, resource: &R) -> bool {
        self.capacity(host, index, resource) > 0
    }
    /// Puts up to `amount` of `resource` into index `index`: how much went in.
    fn insert(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &R,
        amount: u32,
    ) -> u32;
    /// Takes up to `amount` of `resource` out of index `index`: how much came out. None of
    /// another resource.
    fn extract(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &R,
        amount: u32,
    ) -> u32;
    /// Puts up to `amount` of `resource` in wherever it goes (NeoForge's `insert` without an
    /// index): by default index by index from 0. How much went in.
    fn insert_any(&self, host: &mut dyn Host, tx: &Transaction, resource: &R, amount: u32) -> u32 {
        let mut went = 0u32;
        for index in 0..self.size(host) {
            if went >= amount {
                break;
            }
            went = went.saturating_add(self.insert(host, tx, index, resource, amount - went));
        }
        went
    }
    /// Takes up to `amount` of `resource` out from wherever it is (`extract` without an index):
    /// by default index by index from 0. How much came out.
    fn extract_any(&self, host: &mut dyn Host, tx: &Transaction, resource: &R, amount: u32) -> u32 {
        let mut came = 0u32;
        for index in 0..self.size(host) {
            if came >= amount {
                break;
            }
            came = came.saturating_add(self.extract(host, tx, index, resource, amount - came));
        }
        came
    }
}

/// Moves up to `max` of what `filter` accepts out of `from` into wherever `to` takes it, index by
/// index from 0 (NeoForge's `ResourceHandlerUtil.move`), in transaction `tx`: how much moved.
/// Each move is all or nothing, in a transaction of its own: what `to` won't take stays in
/// `from`. A handler answering more than it was asked is warned once by mod; its move is undone
/// and the moving ends there.
pub fn move_resources<R>(
    host: &mut dyn Host,
    from: &Capability<dyn ResourceHandler<R>>,
    to: &Capability<dyn ResourceHandler<R>>,
    filter: impl Fn(&R) -> bool,
    max: u32,
    tx: &Transaction,
) -> u32 {
    let _ = tx;
    let mut moved = 0;
    for index in 0..from.size(host) {
        while moved < max {
            let Some(resource) = from.resource(host, index).filter(|r| filter(r)) else {
                break;
            };
            // `Err(Some(took))`: `to` took only that much, so the move is undone and tried again
            // with that; `Err(None)`: a handler answered more than it was asked.
            let step = |host: &mut dyn Host, want: u32| {
                host.transaction(|host, tx| {
                    let out = from.extract(host, tx, index, &resource, want);
                    let out = from.checked(out, want, &index).ok_or(None)?;
                    let took = to.insert_any(host, tx, &resource, out);
                    match to.checked(took, out, &index).ok_or(None)? {
                        took if took == out => Ok(out),
                        took => Err(Some(took)),
                    }
                })
            };
            let want = (max - moved).min(from.amount(host, index));
            let went = match step(host, want) {
                Err(Some(took)) if took > 0 => step(host, took),
                answer => answer,
            };
            match went {
                Ok(went) if went > 0 => moved += went,
                Ok(_) | Err(Some(_)) => break,
                Err(None) => return moved,
            }
        }
    }
    moved
}
