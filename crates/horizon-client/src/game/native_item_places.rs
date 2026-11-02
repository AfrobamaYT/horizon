//! The game's item places for native mods (`mod_api::ItemPlace`, arismoko/horizon#41 §7): a
//! player's slots, hands and cursor, and item entities. Mods read and write them through
//! `place_stack` and `set_place_stack`, which come here for these places; a block entity's slots,
//! a stack inside an item and a mod's own places go through their handlers.
use super::Game;
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
    /// removes an item entity); false where `native_game_stack` reads nothing for want of the
    /// place. The player's screen follows with the tick's sync.
    pub(super) fn native_set_game_stack(&mut self, place: &ItemPlace, stack: ItemStack) -> bool {
        if !self.places_simulated() {
            return false;
        }
        if let ItemPlace::ItemEntity { id } = place {
            let dimension = self.dim.id();
            return self.native_set_item_entity(dimension, *id, stack);
        }
        let Some(player) = place_player(place) else {
            return false;
        };
        let mut written = false;
        self.native_as_player(player, |g| {
            if let Some(slot) = g.player_place(place) {
                *slot = (stack.count > 0).then_some(stack);
                written = true;
            }
        });
        written
    }
    /// Where an item that leaves a place goes when nothing holds it: a player's into their
    /// inventory, else thrown from their eyes (`ItemUtils.createFilledResult`); dropped where an
    /// item entity is, or at the middle of a block entity's block. What went nowhere: all of it
    /// for a player who isn't here, a gone item entity, and on a client.
    pub(super) fn native_game_rest(
        &mut self,
        place: &ItemPlace,
        rest: ItemStack,
    ) -> Option<ItemStack> {
        if !self.places_simulated() {
            return Some(rest);
        }
        let at = match place {
            ItemPlace::ItemEntity { id } => {
                let Some(e) = self.dropped.iter().find(|e| e.id == *id) else {
                    return Some(rest);
                };
                [e.position.x, e.position.y, e.position.z]
            }
            ItemPlace::BlockEntity { pos, .. } => pos.map(|p| f64::from(p) + 0.5),
            _ => {
                let Some(player) = place_player(place) else {
                    return Some(rest);
                };
                let mut rest = Some(rest);
                self.native_as_player(player, |g| {
                    if let Some(left) = rest.take().and_then(|r| g.give(r)) {
                        g.player_drop(left);
                    }
                });
                return rest;
            }
        };
        // `Block.popResource`'s default pickup delay.
        self.spawn_item(rest, at, [0.0; 3], 10);
        None
    }
}
