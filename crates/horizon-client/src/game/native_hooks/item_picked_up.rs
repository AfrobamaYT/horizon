use super::*;
use horizon_client::entities::ItemEntity;

impl Game<'_> {
    /// `player` touches `count` of `item` on the ground
    /// (`ItemEntity.playerTouch`), before its pickup delay and target are
    /// checked: it is asked on every tick the two touch, even while the item
    /// can't be taken. Leave `can_pickup` at `Default` for vanilla's rule,
    /// set `True` to take it despite its delay or another player's target
    /// (a full inventory still leaves it), or `False` to leave it there: the
    /// stack isn't offered again for 10 ticks. The last mod to set it
    /// decides. Server-side.
    pub(in crate::game) fn native_item_picked_up(
        &mut self,
        player: u128,
        item: &str,
        count: u32,
        can_pickup: &mut TriState,
    ) {
        self.fire_item_picked_up(player, item, count, can_pickup);
    }

    /// `item_picked_up` for the acting player touching the ground `stack`:
    /// the mods' answer, `Default` when no mod listens.
    pub(in crate::game) fn native_pickup_answer(&mut self, stack: &ItemStack) -> TriState {
        let mut can_pickup = TriState::Default;
        if mod_api::__private::listened(mod_api::HookId::ItemPickedUp) {
            let player = self.native_acting_id();
            self.native_item_picked_up(player, &stack.item, stack.count, &mut can_pickup);
        }
        can_pickup
    }

    /// Whether the acting player, `player`, takes the ground `item` they
    /// touch (`ItemEntity.playerTouch`).
    pub(in crate::game) fn native_pickup_allowed(
        &mut self,
        item: &mut ItemEntity,
        player: [i32; 4],
    ) -> bool {
        let answer = self.native_pickup_answer(&item.stack);
        Self::pickup_decision(&mut item.pickup_delay, item.target, player, answer)
    }

    /// `ItemEntity.playerTouch`'s rule for `player`: no pickup delay, and no
    /// target or `player` as the target, unless `answer` forces it either
    /// way. A refusal waits at least 10 ticks before the item is offered
    /// again.
    pub(in crate::game) fn pickup_decision(
        pickup_delay: &mut i64,
        target: Option<[i32; 4]>,
        player: [i32; 4],
        answer: TriState,
    ) -> bool {
        if answer == TriState::False {
            *pickup_delay = (*pickup_delay).max(10);
            return false;
        }
        answer.or(*pickup_delay == 0 && target.is_none_or(|t| t == player))
    }
}
