//! Bulk item storage: [`STORAGE`], a block's items by kind, in 64-bit amounts and without slots,
//! for the stores, pipes and networks of a large base. Every block entity with slots answers it
//! through the game's own adapter over [`ITEMS`], so a pipe reaches a chest and a mod's drive
//! through one capability; a mod's bulk store (drawers, a drive, a vault) answers it itself, one
//! entry per kind however many items it holds.
//!
//! The rule its handlers keep: a question about one kind, and a move of one kind, cost the same
//! whatever is stored and however many items move. A store is listed a page at a time
//! ([`ItemStorage::page`]), never whole in one call.
use crate::content::resource::{Ends, move_once};
use crate::{
    BlockEntityRef, BlockSide, Capability, CapabilityKey, Direction, ITEMS, ItemResource,
    LevelRef, Overanswered, ResourceHandler, ServerHost, Transaction,
};
use std::marker::PhantomData;

/// A block's items by kind: how much of one kind it holds and has room for, and its writes, by
/// kind and in transactions, as a [`ResourceHandler`] is written. Stateless and given the host,
/// as every capability handler is. Amounts are 64-bit: a million items is one call.
pub trait ItemStorage {
    /// How much of `resource` it holds.
    fn amount(&self, host: &mut dyn ServerHost, resource: &ItemResource) -> u64;
    /// How much of `resource` an insert now would take: what a router plans by, not a promise
    /// (an insert may still take less).
    fn space_for(&self, host: &mut dyn ServerHost, resource: &ItemResource) -> u64;
    /// Puts up to `amount` of `resource` in, in transaction `tx`: how much went in.
    fn insert(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        resource: &ItemResource,
        amount: u64,
    ) -> u64;
    /// Takes up to `amount` of `resource` out, in transaction `tx`: how much came out.
    fn extract(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        resource: &ItemResource,
        amount: u64,
    ) -> u64;
    /// Appends up to `limit` of what it holds, from `cursor` on, with their amounts, and moves
    /// `cursor` past them; true when the listing reached its end, `cursor` then back at 0. A
    /// listing changed between pages may pass a kind over or give it twice.
    fn page(
        &self,
        host: &mut dyn ServerHost,
        cursor: &mut usize,
        limit: usize,
        out: &mut Vec<(ItemResource, u64)>,
    ) -> bool;
}

/// Items by kind: a block entity's [`ItemStorage`] as one side reaches it. The game answers it
/// for every block entity [`ITEMS`] reaches, from those slots; a mod's bulk store answers it
/// with a provider of its own (`block_entity_capability(&STORAGE, ..)`). A block with slots and
/// a store of its own registers its provider `before_game`, or the slots answer first.
pub const STORAGE: CapabilityKey<dyn ItemStorage> = CapabilityKey {
    name: "horizon:storage",
    default: Some(|host, at| {
        let slots = at.level.capability(host, &ITEMS, at.pos, at.side)?;
        Some(Box::new(Slots(slots)) as Box<dyn ItemStorage>)
    }),
    handler: PhantomData,
};

/// The game's own [`STORAGE`]: the slots [`ITEMS`] reaches, asked by kind. It inserts as
/// `SimpleContainer.addItem` orders it: the slots that hold the kind first, then the empty ones.
struct Slots(Capability<dyn ResourceHandler<ItemResource>>);

impl Slots {
    /// What slot `index` holds when it holds `resource`.
    fn holds(&self, host: &mut dyn ServerHost, index: usize, resource: &ItemResource) -> bool {
        self.0.resource(host, index).as_ref() == Some(resource)
    }
    /// Up to `left` into the slots `pick` chooses, as one slot takes at most `u32::MAX`.
    fn fill(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        resource: &ItemResource,
        left: &mut u64,
        pick: impl Fn(&mut dyn ServerHost, usize) -> bool,
    ) {
        for index in 0..self.0.size(host) {
            if *left == 0 {
                return;
            }
            if pick(host, index) {
                let ask = (*left).min(u32::MAX as u64) as u32;
                *left -= self.0.insert(host, tx, index, resource, ask).min(ask) as u64;
            }
        }
    }
}

impl ItemStorage for Slots {
    fn amount(&self, host: &mut dyn ServerHost, resource: &ItemResource) -> u64 {
        let mut held = 0;
        for index in 0..self.0.size(host) {
            if self.holds(host, index, resource) {
                held += self.0.amount(host, index) as u64;
            }
        }
        held
    }
    fn space_for(&self, host: &mut dyn ServerHost, resource: &ItemResource) -> u64 {
        (0..self.0.size(host))
            .map(|index| match self.0.resource(host, index) {
                Some(held) if held == *resource => self
                    .0
                    .capacity(host, index, resource)
                    .saturating_sub(self.0.amount(host, index))
                    as u64,
                Some(_) => 0,
                None if self.0.is_valid(host, index, resource) => {
                    self.0.capacity(host, index, resource) as u64
                }
                None => 0,
            })
            .sum()
    }
    fn insert(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        resource: &ItemResource,
        amount: u64,
    ) -> u64 {
        let mut left = amount;
        self.fill(host, tx, resource, &mut left, |host, index| {
            self.holds(host, index, resource)
        });
        self.fill(host, tx, resource, &mut left, |host, index| {
            self.0.resource(host, index).is_none()
        });
        amount - left
    }
    fn extract(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        resource: &ItemResource,
        amount: u64,
    ) -> u64 {
        let mut left = amount;
        for index in 0..self.0.size(host) {
            if left == 0 {
                break;
            }
            if self.holds(host, index, resource) {
                let ask = left.min(u32::MAX as u64) as u32;
                left -= self.0.extract(host, tx, index, resource, ask).min(ask) as u64;
            }
        }
        amount - left
    }
    fn page(
        &self,
        host: &mut dyn ServerHost,
        cursor: &mut usize,
        limit: usize,
        out: &mut Vec<(ItemResource, u64)>,
    ) -> bool {
        let size = self.0.size(host);
        let mut given = 0;
        while *cursor < size && given < limit {
            if let Some(resource) = self.0.resource(host, *cursor) {
                out.push((resource, self.0.amount(host, *cursor) as u64));
                given += 1;
            }
            *cursor += 1;
        }
        let end = *cursor >= size;
        if end {
            *cursor = 0;
        }
        end
    }
}

/// Moves up to `max` of `resource` out of `from` into `to`, in transaction `tx`: how much moved.
/// As one step of [`move_resources`](crate::move_resources): all or nothing in a transaction of
/// its own, a move `to` takes only part of tried again with that part, and a handler answering
/// more than it was asked ending it with [`Overanswered`]. Four handler calls whether one item
/// moves or a million, and none at all when `from` has none or `to` has no room.
pub fn transfer(
    host: &mut dyn ServerHost,
    from: &Capability<dyn ItemStorage>,
    to: &Capability<dyn ItemStorage>,
    resource: &ItemResource,
    max: u64,
    tx: &Transaction,
) -> Result<u64, Overanswered> {
    let _ = tx;
    let want = max
        .min(from.amount(host, resource))
        .min(to.space_for(host, resource));
    if want == 0 {
        return Ok(0);
    }
    move_once(
        host,
        Ends {
            from,
            to,
            at: resource,
        },
        want,
        |host, tx, want| from.extract(host, tx, resource, want),
        |host, tx, out| to.insert(host, tx, resource, out),
    )
}

/// Moves up to `max` of what `filter` accepts out of `from` into `to` (NeoForge's
/// `ResourceHandlerUtil.move`, by kind), in transaction `tx`: how much moved. `from` is listed a
/// page at a time and each kind moved with [`transfer`]; a pass that moved something is followed
/// by another, as a kind emptied mid-listing can shift another past the cursor.
pub fn transfer_matching(
    host: &mut dyn ServerHost,
    from: &Capability<dyn ItemStorage>,
    to: &Capability<dyn ItemStorage>,
    filter: impl Fn(&ItemResource) -> bool,
    max: u64,
    tx: &Transaction,
) -> Result<u64, Overanswered> {
    let mut moved = 0;
    let mut page = Vec::new();
    loop {
        let (mut cursor, mut moved_in_pass) = (0, 0);
        loop {
            page.clear();
            let end = from.page(host, &mut cursor, 64, &mut page);
            for (resource, _) in page.iter().filter(|(resource, _)| filter(resource)) {
                if moved == max {
                    return Ok(moved);
                }
                let went = transfer(host, from, to, resource, max - moved, tx)?;
                moved += went;
                moved_in_pass += went;
            }
            if end {
                break;
            }
        }
        if moved_in_pass == 0 || moved == max {
            return Ok(moved);
        }
    }
}

/// A block capability looked up once and kept (NeoForge's `BlockCapabilityCache`), for a pipe
/// that asks its neighbours every tick: the handler is kept while the block entity at its
/// position is the same one, and looked up again once that one is replaced, removed or unloaded.
/// A provider whose answer changes without that (a machine's side configuration) is told with
/// [`invalidate`](Self::invalidate).
// no wire: holds an owned handler a lookup made in this process; a mod keeps it, never sends it.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockCapabilityCache<T: ?Sized + 'static> {
    key: &'static CapabilityKey<T>,
    at: BlockSide,
    kept: Option<(Option<BlockEntityRef>, Option<Capability<T>>)>,
}

impl<T: ?Sized + 'static> BlockCapabilityCache<T> {
    /// A cache of `key` at `pos` in `level`, reached through `side`; nothing is looked up yet.
    pub fn new(
        key: &'static CapabilityKey<T>,
        level: LevelRef,
        pos: [i32; 3],
        side: Option<Direction>,
    ) -> Self {
        Self {
            key,
            at: BlockSide { level, pos, side },
            kept: None,
        }
    }
    /// The handler there, if any: the kept one while its block entity is the same, else looked
    /// up again ([`LevelRef::capability`]).
    pub fn get(&mut self, host: &mut dyn ServerHost) -> Option<&Capability<T>> {
        let entity = host.block_entity(self.at.level, self.at.pos);
        if !matches!(&self.kept, Some((kept, _)) if *kept == entity) {
            let found = self
                .at
                .level
                .capability(host, self.key, self.at.pos, self.at.side);
            self.kept = Some((entity, found));
        }
        self.kept.as_ref()?.1.as_ref()
    }
    /// Forgets the kept handler, so the next [`get`](Self::get) looks it up again.
    pub fn invalidate(&mut self) {
        self.kept = None;
    }
    /// Where it looks.
    pub fn at(&self) -> BlockSide {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::ItemsHost;
    use crate::testing::FakeHost;
    use crate::{HostTransaction, Identifier, ItemStack, Tag};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    fn host_with(items: &[&str]) -> (FakeHost, Vec<ItemResource>) {
        let mut host = FakeHost::new();
        let ids: Vec<_> = items.iter().map(|id| Identifier::parse(id).unwrap()).collect();
        host.registries.resolve_items(ids.clone()).unwrap();
        for id in items {
            host.registries
                .item_component(id, "minecraft:max_stack_size", Tag::Int(64));
        }
        let resources = ids
            .iter()
            .map(|id| ItemResource::of(&ItemStack::new(host.item_key(id).unwrap(), 1).unwrap()))
            .collect();
        (host, resources)
    }

    fn chest(host: &mut FakeHost, pos: [i32; 3], slots: &[Option<(&ItemResource, u32)>]) {
        let entity = host.level.block_entity(pos).with_items(slots.len());
        for (slot, held) in slots.iter().enumerate() {
            entity.items.as_mut().unwrap().slots[slot] = held.map(|(r, n)| r.to_stack(n).unwrap());
        }
    }

    /// A bulk store in memory, counting the calls made to it: no transactions, for the cases
    /// that never abort.
    #[derive(Default)]
    struct Bulk {
        held: RefCell<HashMap<ItemResource, u64>>,
        room: u64,
        calls: Cell<u32>,
    }
    struct Shared(std::rc::Rc<Bulk>);
    impl ItemStorage for Shared {
        fn amount(&self, _: &mut dyn ServerHost, r: &ItemResource) -> u64 {
            self.0.calls.set(self.0.calls.get() + 1);
            self.0.held.borrow().get(r).copied().unwrap_or(0)
        }
        fn space_for(&self, _: &mut dyn ServerHost, _: &ItemResource) -> u64 {
            self.0.calls.set(self.0.calls.get() + 1);
            self.0.room - self.0.held.borrow().values().sum::<u64>()
        }
        fn insert(&self, h: &mut dyn ServerHost, _: &Transaction, r: &ItemResource, n: u64) -> u64 {
            let went = n.min(self.space_for(h, r));
            *self.0.held.borrow_mut().entry(r.clone()).or_default() += went;
            went
        }
        fn extract(&self, h: &mut dyn ServerHost, _: &Transaction, r: &ItemResource, n: u64) -> u64 {
            let came = n.min(self.amount(h, r));
            *self.0.held.borrow_mut().entry(r.clone()).or_default() -= came;
            came
        }
        fn page(
            &self,
            _: &mut dyn ServerHost,
            cursor: &mut usize,
            _: usize,
            out: &mut Vec<(ItemResource, u64)>,
        ) -> bool {
            out.extend(self.0.held.borrow().iter().map(|(r, n)| (r.clone(), *n)));
            *cursor = 0;
            true
        }
    }
    fn bulk(room: u64, held: &[(&ItemResource, u64)]) -> (std::rc::Rc<Bulk>, Capability<dyn ItemStorage>) {
        let store = std::rc::Rc::new(Bulk {
            held: RefCell::new(held.iter().map(|(r, n)| ((*r).clone(), *n)).collect()),
            room,
            calls: Cell::new(0),
        });
        let handler = Capability {
            handler: Box::new(Shared(store.clone())) as Box<dyn ItemStorage>,
            owner: None,
        };
        (store, handler)
    }

    #[test]
    fn the_game_answers_storage_for_slots_by_kind() {
        let (mut host, r) = host_with(&["minecraft:stone", "minecraft:dirt"]);
        let (stone, dirt) = (&r[0], &r[1]);
        chest(&mut host, [0, 64, 0], &[Some((stone, 40)), Some((dirt, 5)), Some((stone, 10)), None]);
        let level = host.level.level();
        let host: &mut dyn ServerHost = &mut host;
        let storage = level.capability(host, &STORAGE, [0, 64, 0], None).unwrap();
        assert_eq!(storage.amount(host, stone), 50);
        assert_eq!(storage.amount(host, dirt), 5);
        // 24 and 54 topping up the stone, 64 in the empty slot.
        assert_eq!(storage.space_for(host, stone), 24 + 54 + 64);
        let went = host.transaction(|host, tx| Ok::<_, ()>(storage.insert(host, tx, stone, 500)));
        assert_eq!(went, Ok(142), "no more than the room");
        assert_eq!(storage.amount(host, stone), 192);
        assert_eq!(storage.amount(host, dirt), 5, "the dirt's slot is not touched");
        let came = host.transaction(|host, tx| Ok::<_, ()>(storage.extract(host, tx, stone, 100)));
        assert_eq!(came, Ok(100));
        let mut cursor = 0;
        let mut listed = Vec::new();
        assert!(storage.page(host, &mut cursor, 10, &mut listed));
        assert_eq!(listed.iter().map(|(_, n)| n).sum::<u64>(), 97);
        assert_eq!(cursor, 0);
    }

    #[test]
    fn a_slot_write_aborted_with_its_transaction_is_put_back() {
        let (mut host, r) = host_with(&["minecraft:stone"]);
        chest(&mut host, [0, 64, 0], &[Some((&r[0], 10)), None]);
        let level = host.level.level();
        let host: &mut dyn ServerHost = &mut host;
        let storage = level.capability(host, &STORAGE, [0, 64, 0], None).unwrap();
        let aborted: Result<(), ()> = host.transaction(|host, tx| {
            assert_eq!(storage.insert(host, tx, &r[0], 70), 70);
            Err(())
        });
        assert!(aborted.is_err());
        assert_eq!(storage.amount(host, &r[0]), 10);
    }

    #[test]
    fn a_transfer_moves_billions_in_four_calls() {
        let (mut host, r) = host_with(&["minecraft:cobblestone"]);
        let (from_store, from) = bulk(u64::MAX, &[(&r[0], 5_000_000_000)]);
        let (into_store, into) = bulk(u64::MAX, &[]);
        let host: &mut dyn ServerHost = &mut host;
        let moved = host.transaction(|host, tx| transfer(host, &from, &into, &r[0], u64::MAX, tx));
        assert_eq!(moved.ok(), Some(5_000_000_000));
        assert_eq!(into_store.held.borrow()[&r[0]], 5_000_000_000);
        // amount and space_for to plan, then the extract (with its amount) and the insert
        // (with its space_for).
        assert_eq!(from_store.calls.get() + into_store.calls.get(), 4);
    }

    #[test]
    fn a_transfer_into_slots_takes_what_fits_and_leaves_the_rest() {
        let (mut host, r) = host_with(&["minecraft:stone"]);
        chest(&mut host, [0, 64, 0], &[None, None, None]);
        let level = host.level.level();
        let (from_store, from) = bulk(u64::MAX, &[(&r[0], 1000)]);
        let host: &mut dyn ServerHost = &mut host;
        let into = level.capability(host, &STORAGE, [0, 64, 0], None).unwrap();
        let moved = host.transaction(|host, tx| transfer(host, &from, &into, &r[0], 1000, tx));
        assert_eq!(moved.ok(), Some(192));
        assert_eq!(from_store.held.borrow()[&r[0]], 808);
        let moved = host.transaction(|host, tx| transfer(host, &from, &into, &r[0], 1000, tx));
        assert_eq!(moved.ok(), Some(0), "a full target is not tried");
    }

    #[test]
    fn transfer_matching_moves_only_what_the_filter_takes() {
        let (mut host, r) = host_with(&["minecraft:stone", "minecraft:dirt"]);
        chest(&mut host, [0, 64, 0], &[Some((&r[0], 30)), Some((&r[1], 20)), Some((&r[1], 7))]);
        let level = host.level.level();
        let (into_store, into) = bulk(u64::MAX, &[]);
        let host: &mut dyn ServerHost = &mut host;
        let from = level.capability(host, &STORAGE, [0, 64, 0], None).unwrap();
        let dirt = r[1].clone();
        let moved = host.transaction(|host, tx| {
            transfer_matching(host, &from, &into, |res| *res == dirt, u64::MAX, tx)
        });
        assert_eq!(moved.ok(), Some(27));
        assert_eq!(from.amount(host, &r[0]), 30);
        assert_eq!(into_store.held.borrow().get(&r[0]), None);
    }

    #[test]
    fn a_resource_keys_a_map_by_item_damage_and_components() {
        let (host, r) = host_with(&["minecraft:stone", "minecraft:dirt"]);
        let mut worn = ItemStack::new(host.item_key(&Identifier::parse("minecraft:stone").unwrap()).unwrap(), 9).unwrap();
        worn.set_damage(3);
        let mut counts: HashMap<ItemResource, u64> = HashMap::new();
        for resource in [&r[0], &r[1], &ItemResource::of(&worn), &r[0]] {
            *counts.entry(resource.clone()).or_default() += 1;
        }
        assert_eq!(counts.len(), 3);
        assert_eq!(counts[&r[0]], 2, "equal resources hash alike, whatever their counts were");
    }

    #[test]
    fn a_cache_keeps_its_handler_until_the_block_entity_changes() {
        let (mut host, r) = host_with(&["minecraft:stone"]);
        chest(&mut host, [0, 64, 0], &[Some((&r[0], 5))]);
        let level = host.level.level();
        let mut cache = BlockCapabilityCache::new(&STORAGE, level, [0, 64, 0], None);
        {
            let server: &mut dyn ServerHost = &mut host;
            let storage = cache.get(server).unwrap() as *const _;
            assert_eq!(storage, cache.get(server).unwrap() as *const _, "kept, not looked up again");
        }
        host.level.remove_block_entity([0, 64, 0]);
        assert!(cache.get(&mut host).is_none(), "gone with its block entity");
        chest(&mut host, [0, 64, 0], &[Some((&r[0], 8))]);
        assert!(cache.get(&mut host).is_some(), "found again once one is placed");
    }
}
