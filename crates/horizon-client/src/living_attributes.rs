//! The live attribute map of a living entity, shared by the player, mobs,
//! armor stands and the dragon: vanilla's `AttributeMap` over the type's
//! defaults, the `attributes` NBT list (`AttributeMap.pack` and `apply`) and
//! the item modifiers of `LivingEntity.collectEquipmentChanges`.
//!
//! A player owns its map from the start. The others build theirs on first use
//! (an item with attribute modifiers, a read through a `LivingRef`, the save),
//! because most mobs never need one and a map costs a few KiB.
use crate::Result;
use horizon_client_core::nbt_value::{float, string, text};
use horizon_items::{
    equipment::Equipment,
    items::{ItemStack, Items},
};
use horizon_world::{
    attribute_map::ValueKind,
    attributes::{
        Modifier, Operation, Packed,
        entity::Catalog,
        live::{Identities, LiveAttributes},
    },
    nbt::Tag,
};
use std::{cell::OnceCell, fmt, io, sync::Arc};

/// An entity's attribute instances, as a player has them.
pub type Live = LiveAttributes<Box<dyn Identities>>;

pub(crate) fn attribute_error(error: impl fmt::Debug) -> io::Error {
    io::Error::other(format!("movement attribute: {error:?}"))
}

/// A map over `entity`'s type defaults, with no instance yet
/// (`new AttributeMap(DefaultAttributes.getSupplier(type))`). Holder hashes are
/// the registry order, instance identities count up from 1.
pub fn new_live(catalog: &Arc<Catalog>, entity: &str) -> Result<Live> {
    let hashes: Vec<i32> = (0..catalog.attributes().count() as i32).collect();
    let mut identity = 0i32;
    let next: Box<dyn Identities> = Box::new(move || {
        identity = identity.wrapping_add(1);
        identity
    });
    Ok(LiveAttributes::new(catalog.clone(), entity, &hashes, next).map_err(attribute_error)?)
}

/// `EquipmentSlotGroup.test`: whether an item modifier of slot `group` applies
/// in the worn or held `slot`.
pub fn modifier_applies(group: &str, slot: &str) -> bool {
    group == slot
        || group == "any"
        || (group == "hand" && matches!(slot, "mainhand" | "offhand"))
        || (group == "armor" && matches!(slot, "feet" | "legs" | "chest" | "head"))
}

fn operation_named(name: &str) -> Operation {
    match name {
        "add_multiplied_base" => Operation::AddMultipliedBase,
        "add_multiplied_total" => Operation::AddMultipliedTotal,
        _ => Operation::AddValue,
    }
}

/// Adds (or, with `present` false, removes) a transient modifier: the
/// `removeModifier(id)` then `addTransientModifier` of
/// `LivingEntity.collectEquipmentChanges`. An attribute the entity type does not
/// have is left alone.
pub fn set_modifier(
    attributes: &mut Live,
    attribute: &str,
    id: &str,
    amount: f64,
    operation: &str,
    present: bool,
) -> Result<()> {
    let Some(key) = attributes.key(attribute) else {
        return Ok(());
    };
    let Some(handle) = attributes
        .get_instance(Some(key))
        .map_err(attribute_error)?
    else {
        return Ok(());
    };
    let modifier = Modifier::new(id, amount, operation_named(operation))?;
    attributes
        .edit(&handle, |value, observer| {
            value.remove(modifier.id(), observer)?;
            if present {
                value.add_transient(modifier, observer)?;
            }
            Ok(())
        })
        .map_err(attribute_error)?;
    Ok(())
}

/// `AttributeMap.pack` in `AttributeInstance.Packed.LIST_CODEC`'s form: each
/// instance's id, base and, with `modifiers`, its permanent modifiers.
pub fn pack(attributes: &Live, modifiers: bool) -> Vec<Tag> {
    attributes
        .instances()
        .map(|(_, key, instance)| {
            let packed = instance.pack();
            let mut entry = vec![
                ("id", string(key.name())),
                ("base", Tag::Double(packed.base.to_bits())),
            ];
            // `modifiers` is an optional field whose default is the empty list.
            if modifiers && !packed.modifiers.is_empty() {
                let modifiers = packed
                    .modifiers
                    .iter()
                    .map(|m| {
                        let operation = match m.operation() {
                            Operation::AddValue => "add_value",
                            Operation::AddMultipliedBase => "add_multiplied_base",
                            Operation::AddMultipliedTotal => "add_multiplied_total",
                        };
                        Tag::compound([
                            ("id", string(m.id())),
                            ("amount", Tag::Double(m.amount().to_bits())),
                            ("operation", string(operation)),
                        ])
                    })
                    .collect();
                entry.push(("modifiers", Tag::List(modifiers)));
            }
            Tag::compound(entry)
        })
        .collect()
}

/// `AttributeMap.apply` of a saved `attributes` list: each known attribute
/// takes its saved base and permanent modifiers; unknown ones are skipped.
pub fn apply_saved(attributes: &mut Live, saved: Option<&Tag>) -> Result<()> {
    let Some(Tag::List(list)) = saved else {
        return Ok(());
    };
    for entry in list {
        let Some(key) = text(entry.get("id")).and_then(|id| attributes.key(&id)) else {
            continue;
        };
        let modifiers = match entry.get("modifiers") {
            Some(Tag::List(l)) => l
                .iter()
                .filter_map(|m| {
                    let operation = match text(m.get("operation"))?.as_str() {
                        "add_value" => Operation::AddValue,
                        "add_multiplied_base" => Operation::AddMultipliedBase,
                        "add_multiplied_total" => Operation::AddMultipliedTotal,
                        _ => return None,
                    };
                    Modifier::new(&text(m.get("id"))?, float(m.get("amount"))?, operation).ok()
                })
                .collect(),
            _ => Vec::new(),
        };
        let packed = Packed {
            base: float(entry.get("base")).unwrap_or(0.0),
            modifiers,
        };
        let Some(handle) = attributes
            .get_instance(Some(key))
            .map_err(attribute_error)?
        else {
            continue;
        };
        attributes
            .edit(&handle, |value, observer| value.apply(&packed, observer))
            .map_err(attribute_error)?;
    }
    Ok(())
}

/// The attribute map of a mob, an armor stand or the dragon.
///
/// Nothing is allocated for an entity that wears nothing with modifiers and is
/// not asked: `map` and `worn` stay empty, and `saved` holds only the loaded
/// `attributes` entries nothing else models. Once built, by a read too, the
/// map stays until `invalidate` drops it.
#[derive(Clone, Default)]
pub struct LivingAttributes {
    map: OnceCell<Box<Live>>,
    /// The items whose modifiers the map holds, by `EquipmentSlot` ordinal
    /// (`LivingEntity.lastEquipmentItems`).
    worn: Option<Box<[Option<ItemStack>; 8]>>,
    /// The loaded `attributes` entries, applied when the map is built.
    saved: Option<Box<[Tag]>>,
}
impl fmt::Debug for LivingAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LivingAttributes")
            .field("live", &self.map.get().is_some())
            .field("saved", &self.saved.as_ref().map_or(0, |s| s.len()))
            .finish()
    }
}
impl LivingAttributes {
    /// Keeps the entries of a loaded `attributes` list that `keep` accepts
    /// (`LivingEntity.readAdditionalSaveData`), for the map to take.
    pub fn load(&mut self, saved: Option<&Tag>, keep: impl Fn(&str) -> bool) {
        let Some(Tag::List(list)) = saved else {
            return;
        };
        let kept: Vec<Tag> = list
            .iter()
            .filter(|e| text(e.get("id")).is_some_and(|id| keep(&id)))
            .cloned()
            .collect();
        self.saved = (!kept.is_empty()).then(|| kept.into_boxed_slice());
        self.invalidate();
    }

    /// Forgets the loaded attributes the entity type has no instance of, as
    /// `AttributeMap.apply` skips them and `pack` never writes them.
    pub fn retain_supplier(&mut self, catalog: &Catalog, entity: &str) {
        let Some(saved) = self.saved.as_deref() else {
            return;
        };
        let kept: Vec<Tag> = saved
            .iter()
            .filter(|e| {
                text(e.get("id"))
                    .and_then(|id| catalog.id(&id))
                    .is_some_and(|id| catalog.default_base(entity, id).is_some())
            })
            .cloned()
            .collect();
        self.saved = (!kept.is_empty()).then(|| kept.into_boxed_slice());
    }

    /// Whether the map holds an item's modifiers, which a slot emptied since
    /// has to take away.
    pub fn has_worn(&self) -> bool {
        self.worn
            .as_ref()
            .is_some_and(|w| w.iter().any(Option::is_some))
    }

    /// Drops the map, because what it was built from changed: the next use
    /// builds it again and the next equipment pass puts the item modifiers back.
    pub fn invalidate(&mut self) {
        self.map.take();
        self.worn = None;
    }

    /// The map, if it is built.
    pub fn built(&self) -> Option<&Live> {
        self.map.get().map(|map| &**map)
    }

    fn build(&self, catalog: &Arc<Catalog>, entity: &str, seed: Vec<Tag>) -> Result<Live> {
        let mut live = new_live(catalog, entity)?;
        apply_saved(&mut live, Some(&Tag::List(seed)))?;
        if let Some(saved) = &self.saved {
            apply_saved(&mut live, Some(&Tag::List(saved.to_vec())))?;
        }
        Ok(live)
    }

    /// `AttributeMap.getValue` or `getBaseValue` of `name`: None for an
    /// attribute the entity type does not have. `seed` is what the entity keeps
    /// of its attributes elsewhere, in the `attributes` list's form; it is only
    /// asked for when the map is not built yet. The map built for the read is
    /// kept, so the next read allocates nothing.
    pub fn read(
        &self,
        catalog: &Arc<Catalog>,
        entity: &str,
        name: &str,
        kind: ValueKind,
        seed: impl FnOnce() -> Vec<Tag>,
    ) -> Option<f64> {
        let map = match self.map.get() {
            Some(map) => map,
            None => {
                let built = self.build(catalog, entity, seed()).ok()?;
                self.map.get_or_init(|| Box::new(built))
            }
        };
        map.value(Some(map.key(name)?), kind).ok()
    }

    /// `LivingEntity.collectEquipmentChanges` for the item modifiers: every
    /// slot, in `EquipmentSlot` order, whose item is not the one the map
    /// holds loses the old item's modifiers; then the changed slots, in the
    /// same order, take the new item's. Nothing is allocated when no slot
    /// changed.
    pub fn collect_equipment_changes(
        &mut self,
        catalog: &Arc<Catalog>,
        entity: &str,
        equipment: &Equipment,
        items: &Items,
        seed: impl FnOnce() -> Vec<Tag>,
    ) -> Result<()> {
        let item = |slot: usize| equipment.get(slot);
        let mut changed = 0u8;
        for slot in 0..8 {
            let was = self.worn.as_ref().and_then(|w| w[slot].as_ref());
            // `LivingEntity.equipmentHasChanged`: `!ItemStack.matches`.
            let same = match (item(slot), was) {
                (Some(now), Some(was)) => now.matches(was),
                (now, was) => now.is_none() && was.is_none(),
            };
            if !same {
                changed |= 1 << slot;
            }
        }
        if changed == 0 {
            return Ok(());
        }
        let mut seed = Some(seed);
        let mut worn = self.worn.take().unwrap_or_default();
        // `stopLocationBasedEffects`: the old items' modifiers go first.
        for slot in 0..8 {
            if changed & (1 << slot) == 0 {
                continue;
            }
            let (Some(map), Some(old)) = (self.map.get_mut(), worn[slot].take()) else {
                worn[slot] = None;
                continue;
            };
            apply_item(map, &old, slot, items, false)?;
        }
        for slot in 0..8 {
            if changed & (1 << slot) == 0 {
                continue;
            }
            worn[slot] = item(slot).cloned();
            let Some(new) = &worn[slot] else {
                continue;
            };
            if new.attribute_modifiers(items).is_empty() {
                continue;
            }
            if self.map.get().is_none() {
                let seed = seed.take().map_or_else(Vec::new, |f| f());
                let built = self.build(catalog, entity, seed)?;
                self.map.get_or_init(|| Box::new(built));
            }
            if let Some(map) = self.map.get_mut() {
                apply_item(map, new, slot, items, true)?;
            }
        }
        self.worn = Some(worn);
        Ok(())
    }

    /// Writes what the map and the loaded entries say into a saved
    /// `attributes` list, by attribute id: `AttributeMap.pack`, permanent
    /// modifiers only (the item modifiers are transient).
    pub fn save_into(&self, list: &mut Vec<Tag>) {
        let mut put = |entry: Tag| {
            let id = text(entry.get("id"));
            match list.iter_mut().find(|t| text(t.get("id")) == id) {
                Some(t) => *t = entry,
                None => list.push(entry),
            }
        };
        for entry in self.saved.iter().flat_map(|s| s.iter()) {
            put(entry.clone());
        }
        if let Some(map) = self.map.get() {
            for entry in pack(map, true) {
                put(entry);
            }
        }
    }
}

fn apply_item(
    map: &mut Live,
    stack: &ItemStack,
    slot: usize,
    items: &Items,
    present: bool,
) -> Result<()> {
    let slot_name = horizon_items::equipment::SLOT_NAMES[slot];
    for horizon_items::item_data::ModifierRow {
        attribute,
        id,
        amount,
        operation,
        slot: group,
    } in stack.attribute_modifiers(items).iter()
    {
        if modifier_applies(group, slot_name) {
            set_modifier(map, attribute, id, *amount, operation, present)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Held items apply their main-hand and off-hand modifiers (#4118, #4777).
    #[test]
    fn equipment_modifiers_follow_their_slot_group() {
        assert!(modifier_applies("mainhand", "mainhand"));
        assert!(modifier_applies("hand", "mainhand"));
        assert!(modifier_applies("any", "mainhand"));
        assert!(!modifier_applies("armor", "mainhand"));
        assert!(!modifier_applies("mainhand", "head"));
        assert!(modifier_applies("armor", "chest"));
        assert!(!modifier_applies("hand", "feet"));
        assert!(modifier_applies("offhand", "offhand"));
        assert!(modifier_applies("hand", "offhand"));
        assert!(modifier_applies("any", "offhand"));
        assert!(!modifier_applies("armor", "offhand"));
        assert!(!modifier_applies("mainhand", "offhand"));
        assert!(!modifier_applies("offhand", "mainhand"));
        // The body and saddle slots are not armor (`EquipmentSlotGroup.ARMOR`).
        assert!(modifier_applies("body", "body"));
        assert!(modifier_applies("saddle", "saddle"));
        assert!(!modifier_applies("armor", "body"));
        assert!(modifier_applies("any", "saddle"));
    }
}
