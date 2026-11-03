//! The game's item places for native mods (`mod_api::ItemPlace`, arismoko/horizon#41 §7): a
//! player's slots, hands and cursor, and item entities. Mods read and write them through
//! `place_stack` and `set_place_stack`, which come here for these places; a block entity's slots,
//! a stack inside an item and a mod's own places go through their handlers.
use super::{Game, minecarts::CartKind, powder_snow::HeldSlot, shield::OFFHAND_SLOT};
use horizon_client::{cauldron, first_person::Hand, items::ItemStack, mod_api::ItemPlace};

/// The player a place belongs to, if a player's.
fn place_player(place: &ItemPlace) -> Option<u128> {
    match place {
        ItemPlace::PlayerSlot { player, .. }
        | ItemPlace::PlayerHand { player, .. }
        | ItemPlace::PlayerCursor { player } => Some(*player),
        _ => None,
    }
}

impl Game<'_> {
    /// The inventory of entity `id` in the level in place, if it has one: a chest or hopper
    /// minecart's (the open menu's copy while one shows it, as `cart_slots`), a chest boat's, or
    /// a chested donkey's, mule's or llama's (`AbstractHorse.inventory`, `getInventoryColumns`
    /// rows of three). With where it stands, for what a split throws.
    pub(super) fn entity_slots(&mut self, id: u32) -> Option<(&mut [Option<ItemStack>], [f64; 3])> {
        let key = (i32::MAX - 15, super::minecarts::CART_KEY_Y, id as i32);
        let at = |p: &horizon_core::geometry::Vec3| [p.x, p.y, p.z];
        let container = |c: &super::minecarts::Minecart| {
            c.id == id && matches!(c.kind, CartKind::Chest | CartKind::Hopper)
        };
        let position = if let Some(c) = self.carts.iter().find(|c| container(c)) {
            at(&c.position)
        } else if let Some(b) = self.boats.iter().find(|b| b.id == id && b.chest) {
            at(&b.position)
        } else {
            let m = self
                .mobs
                .iter_mut()
                .find(|m| m.id == id && m.extra.chested)?;
            let size = super::mount_menu::mount_columns(m) * 3;
            if m.extra.items.len() < size {
                m.extra.items.resize(size, None);
            }
            return Some((&mut m.extra.items[..size], at(&m.position)));
        };
        if self.carts.iter().any(container) {
            return Some((self.cart_slots(id)?, position));
        }
        if let Some(super::BlockEntity::Chest(open)) = self.world.block_entities.get_mut(&key) {
            return Some((open, position));
        }
        let b = self.boats.iter_mut().find(|b| b.id == id)?;
        Some((&mut b.items, position))
    }
    /// Whether this game simulates a world: a client's copy has no places to write.
    fn places_simulated(&self) -> bool {
        self.serving.is_some() || self.options.dedicated.is_some()
    }
    /// The slot of the player in place that `place` names.
    fn player_place(&mut self, place: &ItemPlace) -> Option<&mut Option<ItemStack>> {
        match place {
            ItemPlace::PlayerSlot { slot, .. } => self.native_carried_slot(*slot),
            ItemPlace::PlayerHand {
                hand: Hand::Main, ..
            } => self.inventory.get_mut(self.selected),
            ItemPlace::PlayerHand {
                hand: Hand::Off, ..
            } => Some(&mut self.offhand),
            ItemPlace::PlayerCursor { .. } => Some(&mut self.carried),
            _ => None,
        }
    }
    /// The stack at a player's or an item entity's place; none for an empty one, a player who
    /// isn't here or is dead, an item entity not in the level in place, any other place
    /// (`place_stack` reads those), and on a client.
    pub(super) fn native_game_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        if !self.places_simulated() {
            return None;
        }
        if let ItemPlace::ItemEntity { id } = place {
            return self
                .dropped
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.stack.clone());
        }
        if let ItemPlace::EntitySlot { entity, slot } = place {
            let (slots, _) = self.entity_slots(*entity)?;
            return slots.get(*slot).cloned().flatten().filter(|s| s.count > 0);
        }
        let mut stack = None;
        self.native_as_player(place_player(place)?, |g| {
            stack = g.player_place(place).and_then(|slot| slot.clone());
        });
        stack.filter(|s| s.count > 0)
    }
    /// Puts `stack` at a player's or an item entity's place (a count of 0 empties it, and
    /// removes an item entity); false, with nothing changed, where `native_game_stack` reads
    /// nothing for want of the place. With `one_by_one`, `stack` stands for one item of what is
    /// there: from a player's hand it goes as `ItemUtils.createFilledResult` puts a filled
    /// bucket (in creative the hand is kept); from a player's other places it goes into their
    /// inventory, else is dropped as `Player.drop(stack, false)`; from an item entity of more
    /// than one, it's dropped by it. These never refuse, so `simulate` only asks whether the
    /// place is there. The player's screen follows with the tick's sync.
    pub(super) fn native_set_game_stack(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        one_by_one: bool,
        simulate: bool,
    ) -> bool {
        if !self.places_simulated() {
            return false;
        }
        if let ItemPlace::ItemEntity { id } = place {
            let Some(e) = self.dropped.iter().find(|e| e.id == *id) else {
                return false;
            };
            if simulate {
                return true;
            }
            let dimension = self.dim.id();
            if !one_by_one || e.stack.count <= 1 {
                return self.native_set_item_entity(dimension, *id, stack);
            }
            let at = [e.position.x, e.position.y, e.position.z];
            let kept = ItemStack {
                count: e.stack.count - 1,
                ..e.stack.clone()
            };
            if !self.native_set_item_entity(dimension, *id, kept) {
                return false;
            }
            // `Block.popResource`'s default pickup delay.
            if stack.count > 0 {
                self.spawn_item(stack, at, [0.0; 3], 10);
            }
            return true;
        }
        if let ItemPlace::EntitySlot { entity, slot } = place {
            return self.set_entity_slot(*entity, *slot, stack, one_by_one, simulate);
        }
        let Some(player) = place_player(place) else {
            return false;
        };
        let mut written = false;
        self.native_as_player(player, |g| {
            written = g.player_place(place).is_some();
            if !written || simulate {
                return;
            }
            g.journal_player(player);
            if one_by_one && let ItemPlace::PlayerHand { hand, .. } = place {
                let slot = match hand {
                    Hand::Main => g.selected,
                    Hand::Off => OFFHAND_SLOT,
                };
                let mut held = HeldSlot {
                    game: g,
                    slot,
                    dropper: Some(player),
                };
                cauldron::create_filled_result(&mut held, stack, true);
                return;
            }
            let Some(slot) = g.player_place(place) else {
                return;
            };
            let split = one_by_one && slot.as_ref().is_some_and(|held| held.count > 1);
            if !split {
                *slot = (stack.count > 0).then_some(stack);
                return;
            }
            if let Some(held) = slot.as_mut() {
                held.count -= 1;
            }
            if let Some(rest) = (stack.count > 0).then(|| g.give(stack)).flatten() {
                g.drop_after_commit(player, rest);
            }
        });
        written
    }
    /// `native_set_game_stack` for slot `slot` of entity `entity`'s inventory: with
    /// `one_by_one` from a stack of more than one, the changed item goes into the inventory's
    /// other slots, else is thrown where the entity stands.
    fn set_entity_slot(
        &mut self,
        entity: u32,
        slot: usize,
        stack: ItemStack,
        one_by_one: bool,
        simulate: bool,
    ) -> bool {
        let Some((slots, _)) = self.entity_slots(entity) else {
            return false;
        };
        if slot >= slots.len() || simulate {
            return slot < slots.len();
        }
        self.journal_entity(entity);
        let Some((slots, at)) = self.entity_slots(entity) else {
            return false;
        };
        let split = one_by_one && slots[slot].as_ref().is_some_and(|held| held.count > 1);
        if !split {
            slots[slot] = (stack.count > 0).then_some(stack);
            return true;
        }
        if let Some(held) = slots[slot].as_mut() {
            held.count -= 1;
        }
        if stack.count == 0 {
            return true;
        }
        let max = self.containers().max_stack(&stack);
        let Some((slots, _)) = self.entity_slots(entity) else {
            return true;
        };
        let mut rest = stack;
        for held in slots.iter_mut() {
            match held {
                Some(h) if h.count < max && h.same_item_same_components(&rest) => {
                    let moved = (max - h.count).min(rest.count);
                    h.count += moved;
                    rest.count -= moved;
                }
                None => {
                    *held = Some(rest.clone());
                    rest.count = 0;
                }
                _ => {}
            }
            if rest.count == 0 {
                return true;
            }
        }
        let pos = at.map(|p| p.floor() as i32);
        self.after_commit(super::transactions::After::Drop(pos, rest));
        true
    }
    /// Drops `stack` at the middle of the block at `pos`, as a block entity's handler gives up
    /// what it can't put back.
    pub(super) fn native_drop_at(&mut self, pos: [i32; 3], stack: ItemStack) {
        if self.places_simulated() && stack.count > 0 {
            // `Block.popResource`'s default pickup delay, at the root commit.
            self.after_commit(super::transactions::After::Drop(pos, stack));
        }
    }
}
