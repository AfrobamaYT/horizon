//! The `player_swap_hands` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0262

pub trait Hooks {
    /// `player` (`name`) pressed the swap-hands key: heard before the main and
    /// off hand items swap (the `SWAP_ITEM_WITH_OFFHAND` player action that
    /// `ServerGamePacketListenerImpl.handlePlayerAction` answers; Bukkit's
    /// `PlayerSwapHandItemsEvent`), not for a spectator. `to_main_hand` starts
    /// as the off hand's stack and `to_off_hand` as the main hand's; set either
    /// to change what that hand gets (`setItemSwappedToMainHand`,
    /// `setItemSwappedToOffHand`), and a client whose stacks changed gets both
    /// slots. Set `cancel` to keep the items where they are: the player's
    /// client, which swapped them at once, gets its slots back. The player's
    /// state is swapped in, so `host.players().acting_player()`,
    /// `PlayerRef::tell` and the item helpers reach them. Once for each press,
    /// on a server: a LAN or dedicated server's players, the single-player
    /// owner and a LAN host. Not heard: the same swap made in an open inventory
    /// (hovering a slot and pressing the key, a menu click).
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        version = 2,
        neoforge = "LivingSwapItemsEvent.Hands"
    )]
    fn player_swap_hands();
}
