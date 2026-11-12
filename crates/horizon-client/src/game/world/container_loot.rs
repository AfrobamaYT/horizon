//! First-access loot shared by blocks and chest vehicles.
use super::Game;
use horizon_client::{containers::BlockEntity, save::PendingContainerLoot};
use horizon_core::{nbt::Tag, random::LegacyRandom};

#[derive(Clone, Copy)]
pub(super) enum LootTarget {
    Block((i32, i32, i32)),
    Cart(u32),
    Boat(u32),
}
impl Game<'_> {
    pub(super) fn loot_inventory_item(
        &mut self,
        key: (i32, i32, i32),
        index: usize,
    ) -> Option<horizon_client::items::ItemStack> {
        self.unpack_loot(LootTarget::Block(key), None);
        self.world
            .block_entities
            .get(&key)?
            .slots()
            .get(index)?
            .clone()
    }
    /// Consumes metadata before callbacks, so reentrant access cannot roll twice.
    /// Player luck/context is added in the next implementation step.
    pub(super) fn unpack_loot(&mut self, target: LootTarget, player: Option<mod_api::PlayerRef>) {
        let (pending, pos, occupied) = match target {
            LootTarget::Block(key) => {
                if !self.world.block_entities.contains_key(&key)
                    || !self
                        .world
                        .chunks
                        .get(&(key.0 >> 4, key.2 >> 4))
                        .and_then(|c| c.saved.as_ref())
                        .is_some_and(|s| s.pending_loot.contains_key(&key))
                {
                    return;
                }
                let mut pending = None;
                self.world
                    .with_chunk_extras(key, |extras| pending = extras.pending_loot.remove(&key));
                let Some(pending) = pending else { return };
                self.world.modified.insert((key.0 >> 4, key.2 >> 4));
                (
                    pending,
                    key,
                    self.world
                        .block_entities
                        .get(&key)
                        .unwrap()
                        .slots()
                        .iter()
                        .any(Option::is_some),
                )
            }
            LootTarget::Cart(id) => {
                let Some(i) = self.carts.iter().position(|c| c.id == id) else {
                    return;
                };
                let Some(pending) = self.carts.update(i, |c| c.loot.take()) else {
                    return;
                };
                self.store_cart_items((i32::MAX - 15, super::minecarts::CART_KEY_Y, id as i32));
                let c = &self.carts[i];
                (
                    pending,
                    (
                        c.position.x.floor() as i32,
                        c.position.y.floor() as i32,
                        c.position.z.floor() as i32,
                    ),
                    c.items.iter().any(Option::is_some),
                )
            }
            LootTarget::Boat(id) => {
                let Some(i) = self.boats.iter().position(|b| b.id == id) else {
                    return;
                };
                let Some(pending) = self
                    .persist
                    .raw
                    .get(&id)
                    .and_then(PendingContainerLoot::from_tag)
                else {
                    return;
                };
                if let Some(Tag::Compound(map)) = self.persist.raw.get_mut(&id) {
                    map.remove(&"LootTable".into());
                    map.remove(&"LootTableSeed".into());
                }
                self.store_cart_items((i32::MAX - 15, super::minecarts::CART_KEY_Y, id as i32));
                let b = &self.boats[i];
                (
                    pending,
                    (
                        b.position.x.floor() as i32,
                        b.position.y.floor() as i32,
                        b.position.z.floor() as i32,
                    ),
                    b.items.iter().any(Option::is_some),
                )
            }
        };
        // Horizon-only: an inventory with both items and a table drops the
        // table on first access and never rolls, even if later emptied.
        if occupied {
            return;
        }
        let table = pending.table_id();
        if player.is_some() {
            self.trigger_loot_table(&table);
        }
        let Some(loot) = self.server_loot.get() else {
            return;
        };
        let biome = self
            .biome_name_at(super::Vec3::new(pos.0 as f64, pos.1 as f64, pos.2 as f64))
            .unwrap_or_default();
        // Missing/zero seed policy remains unchanged; tracked separately in #3690.
        let seed = pending.seed_value().filter(|s| *s != 0).unwrap_or_else(|| {
            horizon_core::random::position_seed(horizon_core::position::BlockPos::new(
                pos.0, pos.1, pos.2,
            ))
        });
        let mut random = LegacyRandom::new(seed);
        let Some(items) = loot.roll_container_with(&table, &mut random, Some(&biome)) else {
            return;
        };
        let fill = |slots: &mut [Option<horizon_client::items::ItemStack>]| {
            let available: Vec<_> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.is_none().then_some(i))
                .collect();
            let mut placed = vec![None; slots.len()];
            loot.place_container_loot(items, &mut placed, available, &mut random);
            for (slot, stack) in slots.iter_mut().zip(placed) {
                if let Some(stack) = stack {
                    *slot = Some(horizon_client::structure_blocks::loot_item_with_components(
                        &stack,
                    ));
                }
            }
        };
        match target {
            LootTarget::Block(key) => {
                if let Some(e) = self.world.block_entities.get_mut(&key) {
                    fill(e.slots_mut());
                }
            }
            LootTarget::Cart(id) => {
                if let Some(i) = self.carts.iter().position(|c| c.id == id) {
                    self.carts.update(i, |c| fill(&mut c.items));
                }
            }
            LootTarget::Boat(id) => {
                if let Some(i) = self.boats.iter().position(|b| b.id == id) {
                    self.boats.update(i, |b| fill(&mut b.items));
                }
            }
        }
        let id = match target {
            LootTarget::Cart(id) | LootTarget::Boat(id) => Some(id),
            _ => None,
        };
        if let Some(id) = id {
            let items = match target {
                LootTarget::Cart(_) => self
                    .carts
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.items.clone()),
                _ => self
                    .boats
                    .iter()
                    .find(|b| b.id == id)
                    .map(|b| b.items.clone()),
            };
            if let (Some(items), Some(BlockEntity::Chest(open))) = (
                items,
                self.world.block_entities.get_mut(&(
                    i32::MAX - 15,
                    super::minecarts::CART_KEY_Y,
                    id as i32,
                )),
            ) {
                *open = items;
            }
        }
    }
}
