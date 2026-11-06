//! The `menu_refs` area: host methods the game implements (`game/host/menu_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `menu_refs` area, implemented by the game and reached through [`ClientHost`].
pub trait MenuRefsClientHost {
    /// Side: client.
    /// The local player in a logged-in client world.
    fn local_player(&self) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "ClientHost::local_player",
            "local_player",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
/// The handles' reads of the `menu_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MenuRefsBackend {
    /// Side: both.
    /// A player's current supported menu. Refuses crafting tables, merchants,
    /// workstations, lecterns, mounts and menus with hidden slots; also refuses
    /// clients, stale players and players whose state is swapped out.
    fn player_container_menu(&self, player: PlayerRef) -> Option<MenuRef> {
        crate::phase::unprovided(
            "Backend::player_container_menu",
            "player_container_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Container id (0 for the player's inventory menu).
    fn menu_container_id(&self, menu: MenuRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::menu_container_id",
            "menu_container_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Registry menu type; the inventory menu has no MenuType in vanilla.
    /// The outer None means an unavailable menu.
    fn menu_kind(&self, menu: MenuRef) -> Option<Option<String>> {
        crate::phase::unprovided(
            "Backend::menu_kind",
            "menu_kind",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Number of slots in AbstractContainerMenu.slots.
    fn menu_slot_count(&self, menu: MenuRef) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::menu_slot_count",
            "menu_slot_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Full stack at a menu index. Outer None means unavailable or out of
    /// range; Some(None) means an empty slot.
    fn menu_slot(&self, menu: MenuRef, slot: usize) -> Option<Option<ItemStack>> {
        crate::phase::unprovided(
            "Backend::menu_slot",
            "menu_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Carried (cursor) stack; outer None means unavailable.
    fn menu_carried(&self, menu: MenuRef) -> Option<Option<ItemStack>> {
        crate::phase::unprovided(
            "Backend::menu_carried",
            "menu_carried",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' client writes of the `menu_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MenuRefsClientBackendMut {
    /// Side: client.
    /// Submit clicks on the local player's live menu through normal prediction
    /// and `ContainerClick`. Slots use the same menu order as `slot`; `None`
    /// is outside (-999). All clicks are validated before any prediction or send.
    /// A drag expands to one complete quick-craft gesture.
    /// Refuses stale menus, invalid slots/hotbar indices, spectators, non-client hosts,
    /// the creative inventory screen and empty batches without changing items.
    /// True means submitted, not accepted by the server: its corrections win.
    fn menu_click(&mut self, menu: MenuRef, clicks: &[MenuClick]) -> bool {
        crate::phase::unprovided(
            "ClientBackendMut::menu_click",
            "menu_click",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Close the local player's open container screen as Escape does
    /// (`AbstractContainerScreen.onClose`, `LocalPlayer.closeContainer`): the
    /// server is told (`ContainerClose`) and the screen goes. Refuses stale
    /// menus, non-client hosts and a menu whose screen is not open, such as
    /// the inventory menu while playing.
    fn menu_close(&mut self, menu: MenuRef) -> bool {
        crate::phase::unprovided(
            "ClientBackendMut::menu_close",
            "menu_close",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Press button `button` of the local player's open menu as a click on
    /// one of its buttons does (`MultiPlayerGameMode.handleInventoryButtonClick`,
    /// `ContainerButtonClick`): the server's menu decides what it does
    /// (`clickMenuButton`, and `menu_button` for a mod's menu). For a client
    /// mod's own input on a menu's screen where `screen_click` doesn't come,
    /// such as a key or the wheel; no sound, that is the screen's. Refuses
    /// stale menus, non-client hosts and the inventory menu, which no server
    /// opened. True means sent, not that the menu took it.
    fn menu_click_button(&mut self, menu: MenuRef, button: i32) -> bool {
        crate::phase::unprovided(
            "ClientBackendMut::menu_click_button",
            "menu_click_button",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
