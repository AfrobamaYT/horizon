use super::*;

impl Game<'_> {
    /// `player` (`name`) pressed the swap-hands key: heard before the main and
    /// off hand items swap (the `SWAP_ITEM_WITH_OFFHAND` player action that
    /// `ServerGamePacketListenerImpl.handlePlayerAction` answers; NeoForge's
    /// `LivingSwapItemsEvent.Hands`), not for a spectator. `to_main_hand` starts
    /// as the off hand's stack and `to_off_hand` as the main hand's; the hands
    /// get what the listeners leave, and a client whose stacks changed gets
    /// both slots. Set `cancel` to keep the items where they are: the player's
    /// client, which swapped them at once, gets its slots back. The player's
    /// state is swapped in, so `native_acting_player`, `native_tell` and the
    /// item helpers reach them. Once for each press, on a server: a LAN or
    /// dedicated server's players, the single-player owner and a LAN host.
    /// Not heard: the same swap made in an open inventory (hovering a slot and
    /// pressing the key, a menu click).
    pub(in crate::game) fn native_player_swap_hands(
        &mut self,
        player: u128,
        name: &str,
        to_main_hand: &mut Option<ItemStack>,
        to_off_hand: &mut Option<ItemStack>,
        cancel: &mut bool,
    ) {
        self.fire_player_swap_hands(player, name, to_main_hand, to_off_hand, cancel);
    }
}
