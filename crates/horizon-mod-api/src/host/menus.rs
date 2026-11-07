//! The `menus` area: host methods the game implements (`game/host/menus.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `menus` area, implemented by the game and reached through [`ServerHost`].
pub trait MenusServerHost {
    /// Side: server.
    /// Opens a chest menu of `rows` rows (1 to 6) titled `title` for the
    /// player, showing `items` (by slot, padded with empty ones), in place of
    /// the menu they had open: its window id.
    fn open_menu(
        &mut self,
        uuid: u128,
        rows: u8,
        title: Text,
        items: Vec<Option<ItemStack>>,
    ) -> Option<i32> {
        crate::phase::unprovided(
            "ServerHost::open_menu",
            "open_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Closes the player's native menu (a `native_menu_closed` follows).
    fn close_menu(&mut self, uuid: u128) {
        crate::phase::unprovided(
            "ServerHost::close_menu",
            "close_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `menus` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait MenusClientHost {
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
/// The handles' reads of the `menus` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait MenusBackend {
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
    /// Side: server.
    /// The storage menu slot `slot` shows (`Slot.container` and `getContainerSlot`): the
    /// player's inventory by `ENTITY_ITEMS` index (`InventoryMenu`'s armour slots 5..9 go head
    /// to feet), a block entity's or an entity's slot, the ender chest's, the crafting grid's
    /// or a mod's chest menu's.
    fn menu_slot_backing(&self, menu: MenuRef, slot: usize) -> Option<MenuSlotBacking> {
        crate::phase::unprovided(
            "Backend::menu_slot_backing",
            "menu_slot_backing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many of `stack` menu slot `slot` holds (`Slot.mayPlace`, then `Slot.getMaxStackSize`):
    /// a furnace's fuel slot one bucket, an armour slot one piece that fits it, a crafting result
    /// none.
    fn menu_slot_holds(&self, menu: MenuRef, slot: usize, stack: &ItemStack) -> Option<u32> {
        crate::phase::unprovided(
            "Backend::menu_slot_holds",
            "menu_slot_holds",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether menu slot `slot` gives up what it holds (`Slot.mayPickup`): bound armour stays
    /// on outside creative.
    fn menu_slot_gives(&self, menu: MenuRef, slot: usize) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::menu_slot_gives",
            "menu_slot_gives",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' client writes of the `menus` area, implemented by the game and reached through [`ClientBackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait MenusClientBackendMut {
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
