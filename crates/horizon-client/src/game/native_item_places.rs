//! The game's item places for native mods (`mod_api::ItemPlace`, arismoko/horizon#41 §7): a
//! player's slots, hands and cursor, and item entities. Mods read and write them through
//! `place_stack` and `set_place_stack`, which come here for these places; a block entity's slots,
//! a stack inside an item and a mod's own places go through their handlers.
use super::{Game, powder_snow::filled_result_split, shield::OFFHAND_SLOT};
use horizon_client::{first_person::Hand, items::ItemStack, mod_api::ItemPlace};

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
    /// bucket (in creative the hand is kept); from a player's other places the same, without
    /// creative's rule; from an item entity of more than one, it's dropped by it. The player's
    /// screen follows with the tick's sync.
    pub(super) fn native_set_game_stack(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        one_by_one: bool,
    ) -> bool {
        if !self.places_simulated() {
            return false;
        }
        if let ItemPlace::ItemEntity { id } = place {
            let Some(e) = self.dropped.iter().find(|e| e.id == *id) else {
                return false;
            };
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
        let Some(player) = place_player(place) else {
            return false;
        };
        let mut written = false;
        self.native_as_player(player, |g| {
            if one_by_one && let ItemPlace::PlayerHand { hand, .. } = place {
                let slot = match hand {
                    Hand::Main => g.selected,
                    Hand::Off => OFFHAND_SLOT,
                };
                g.filled_result_in(slot, stack);
                written = true;
                return;
            }
            let Some(slot) = g.player_place(place) else {
                return;
            };
            written = true;
            if !one_by_one {
                *slot = (stack.count > 0).then_some(stack);
                return;
            }
            let (kept, out) = filled_result_split(slot.take(), stack, false, false);
            *slot = kept;
            if let Some(rest) = out.and_then(|out| g.give(out)) {
                g.drop_plain(rest);
            }
        });
        written
    }
}
