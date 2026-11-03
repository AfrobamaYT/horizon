//! Entity capabilities (arismoko/horizon#41 §7): what a mod registers for an entity type to
//! answer a capability, such as [`ENTITY_ITEMS`], beside the game's own (NeoForge's
//! `Capabilities.Item.ENTITY`). A player's inventory is the game's own answer for a player.
use super::{
    Capability, ComponentId, EntityProvider, EntityRef, Hand, Host, ItemPlace, ItemResource,
    ProvidedBy, ResourceHandler, Tag, Transaction,
};
use std::marker::PhantomData;

/// An entity capability: its name, the handler trait `T` it answers with, and the game's own
/// provider, if the game has one.
pub struct EntityCapabilityKey<T: ?Sized + 'static> {
    pub name: &'static str,
    default: Option<fn(&mut dyn Host, EntityRef) -> Option<Box<T>>>,
    handler: PhantomData<fn() -> Box<T>>,
}

impl<T: ?Sized + 'static> EntityCapabilityKey<T> {
    /// A mod's own entity capability, answered only by the providers registered for it.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            default: None,
            handler: PhantomData,
        }
    }
}

/// An entity's items, as NeoForge 26.3's `Capabilities.Item.ENTITY`. The game answers it for a
/// player with their inventory (`PlayerInventoryWrapper`): indices as vanilla's `Inventory`
/// numbers them, 0..36 the main inventory (0..9 the hotbar), 36..40 the armour from feet to
/// head, 40 the off hand.
pub const ENTITY_ITEMS: EntityCapabilityKey<dyn ResourceHandler<ItemResource>> =
    EntityCapabilityKey {
        name: "horizon:entity_items",
        default: Some(|host, entity| {
            if entity.kind(host)? != "minecraft:player" {
                return None;
            }
            let player = entity.uuid(host)?;
            Some(Box::new(PlayerItems(player)) as Box<dyn ResourceHandler<ItemResource>>)
        }),
        handler: PhantomData,
    };

impl EntityRef {
    /// `key`'s handler for this entity (`Entity.getCapability`): the first provider to answer, of
    /// those mods registered for its type `before_game`, the game's own, and the mods' others,
    /// each in registration order. None for a stale handle and for an entity outside the level
    /// in place: no provider is asked for one.
    pub fn capability<T: ?Sized + 'static>(
        self,
        host: &mut dyn Host,
        key: &EntityCapabilityKey<T>,
    ) -> Option<Capability<T>> {
        if !self.level(host)?.in_place(host) {
            return None;
        }
        let mut game = None;
        let parts = super::providers(ProvidedBy::Entity, key.name, host, |host| self.kind(host));
        super::answered(
            super::ask_providers::<EntityProvider<T>, T>(
                parts,
                host,
                |provider, host| provider(host, self),
                |host| {
                    game = key.default.and_then(|default| default(host, self));
                    game.is_some()
                },
            ),
            game,
        )
    }
}

/// The player inventory's slots, by `Inventory`'s numbers (`PlayerInventoryWrapper`).
const MAIN: usize = 36;
const ARMOUR: [&str; 4] = ["feet", "legs", "chest", "head"];
const OFF_HAND: usize = MAIN + ARMOUR.len();

/// The game's own [`ENTITY_ITEMS`] for player `.0`: each index is the player's
/// [`ItemPlace::PlayerSlot`], written in place and journaled, and takes only what fits there.
struct PlayerItems(u128);

impl PlayerItems {
    fn place(&self, index: usize) -> ItemPlace {
        let slot = match index {
            OFF_HAND => -106,
            index if index >= MAIN => 100 + (index - MAIN) as i32,
            index => index as i32,
        };
        ItemPlace::PlayerSlot {
            player: self.0,
            slot,
        }
    }
    /// Puts up to `amount` of `resource` at `place`, into what fits there and no further: an
    /// empty place or one with the same item, up to `limit`.
    fn fill(
        host: &mut dyn Host,
        tx: &Transaction,
        place: &ItemPlace,
        resource: &ItemResource,
        amount: u32,
        limit: u32,
    ) -> u32 {
        let held = match place.stack(host) {
            Some(held) if !resource.matches(&held) => return 0,
            held => held.map_or(0, |held| held.count),
        };
        match amount.min(limit.saturating_sub(held)) {
            0 => 0,
            room => place.insert(host, tx, resource, room),
        }
    }
}

/// The most of `resource` one slot holds: its `max_stack_size`.
fn max_stack(host: &dyn Host, resource: &ItemResource) -> u32 {
    let id = ComponentId::new("minecraft:max_stack_size").expect("a component id");
    match resource.value(host, &id) {
        Some(Tag::Int(size)) => u32::try_from(size).unwrap_or(0),
        _ => 0,
    }
}

/// Whether `resource` goes in the armour slot `slot` (`feet`...): its `equippable` names that
/// slot and lets a player wear it (`ArmorSlot.mayPlace`, `Equippable.canBeEquippedBy`), its
/// allowed entities a list or one id; a tag of entity types is read as not holding a player.
fn equips(host: &dyn Host, resource: &ItemResource, slot: &str) -> bool {
    let id = ComponentId::new("minecraft:equippable").expect("a component id");
    let Some(equippable) = resource.value(host, &id) else {
        return false;
    };
    let named = |tag: Option<&Tag>, id: &str| match tag {
        Some(Tag::String(s)) => s.to_string_lossy() == id,
        _ => false,
    };
    named(equippable.get("slot"), slot)
        && match equippable.get("allowed_entities") {
            None => true,
            Some(Tag::List(ids)) => ids.iter().any(|id| named(Some(id), "minecraft:player")),
            entities => named(entities, "minecraft:player"),
        }
}

impl ResourceHandler<ItemResource> for PlayerItems {
    fn size(&self, _: &mut dyn Host) -> usize {
        OFF_HAND + 1
    }
    fn resource(&self, host: &mut dyn Host, index: usize) -> Option<ItemResource> {
        self.place(index).resource(host)
    }
    fn amount(&self, host: &mut dyn Host, index: usize) -> u32 {
        self.place(index).amount(host)
    }
    fn capacity(&self, host: &mut dyn Host, index: usize, resource: &ItemResource) -> u32 {
        match index
            .checked_sub(MAIN)
            .and_then(|armour| ARMOUR.get(armour))
        {
            Some(slot) if !equips(host, resource, slot) => 0,
            _ if index > OFF_HAND => 0,
            _ => max_stack(host, resource),
        }
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        let limit = self.capacity(host, index, resource);
        Self::fill(host, tx, &self.place(index), resource, amount, limit)
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        if index > OFF_HAND {
            return 0;
        }
        self.place(index).extract(host, tx, resource, amount)
    }
    /// Where `Inventory.add` puts it: onto the same item in the selected slot, then in the off
    /// hand, then in the main inventory in order, then into its empty slots in order; never
    /// the armour.
    fn insert_any(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        let limit = max_stack(host, resource);
        let selected = ItemPlace::PlayerHand {
            player: self.0,
            hand: Hand::Main,
        };
        let same = [selected, self.place(OFF_HAND)]
            .into_iter()
            .chain((0..MAIN).map(|index| self.place(index)));
        let mut went = 0;
        for place in same {
            if went < amount
                && place
                    .stack(host)
                    .is_some_and(|held| resource.matches(&held))
            {
                went += Self::fill(host, tx, &place, resource, amount - went, limit);
            }
        }
        for index in 0..MAIN {
            let place = self.place(index);
            if went < amount && place.stack(host).is_none() {
                went += Self::fill(host, tx, &place, resource, amount - went, limit);
            }
        }
        went
    }
}
