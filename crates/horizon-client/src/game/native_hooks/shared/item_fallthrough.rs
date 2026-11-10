use super::*;

impl Game<'_> {
    /// Item.use after the same hand's target passed, or for a server UseItem packet.
    pub(in crate::game) fn right_click_item(
        &mut self,
        hand: Hand,
        stack: &ItemStack,
    ) -> Option<mod_api::InteractionResult> {
        if !mod_api::__private::listened(mod_api::HookId::RightClickItem) {
            return None;
        }
        let pos = self
            .target_position(crate::game::commands::Target::Player)?
            .map(|v| v.floor() as i32);
        let (player, player_entity_id) = self.native_clicker();
        let snapshot = stack.try_into_api().ok()?;
        let click = ItemClick::from(mod_api::__private::built::ItemClick {
            side: if self.remote.is_some() {
                LogicalSide::Client
            } else {
                LogicalSide::Server
            },
            dimension: self.dim.level(),
            pos,
            hand,
            stack: &snapshot,
            player,
            player_entity_id,
        });
        let instant = self.use_duration(stack) <= 0;
        let (mut result, mut canceled) = (mod_api::InteractionResult::Pass, false);
        self.native_right_click_item(&click, &mut result, &mut canceled);
        if canceled && instant && result.consumes_action() {
            self.with_hand(hand, |game| game.apply_after_use_components(stack));
        }
        canceled.then_some(result)
    }
}
