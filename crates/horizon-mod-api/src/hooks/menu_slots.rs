//! The `menu_slots` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "menu",
        target = "horizon_client::game::native_menu_refs::Game<'_>::native_menu_slot_backing"
    )]
    fn menu_slot_backing();
    #[helper(
        side = "server",
        receiver = "menu",
        target = "horizon_client::game::native_menu_refs::Game<'_>::native_menu_slot_holds"
    )]
    fn menu_slot_holds();
    #[helper(
        side = "server",
        receiver = "menu",
        target = "horizon_client::game::native_menu_refs::Game<'_>::native_menu_slot_gives"
    )]
    fn menu_slot_gives();
}
