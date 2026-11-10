//! The client nonce is tied to the actual shown mod screen, not just its menu type.
use super::*;
impl Game<'_> {
    pub(in crate::game) fn native_book_placement_window_matches(
        &self,
        window: &horizon_client::recipe_book::placement_sync::Window,
    ) -> bool {
        self.shown_native_screen().is_some_and(|screen| {
            screen.window == window.window
                && screen.layout_revision == window.nonce
                && screen
                    .screen
                    .recipe_book
                    .as_ref()
                    .is_some_and(|key| key.as_str() == window.book.as_str())
        }) && self.native_screen_menu() == Some(window.menu.as_str())
    }
    pub(in crate::game) fn native_book_placement_window(
        &self,
    ) -> Option<horizon_client::recipe_book::placement_sync::Window> {
        let screen = self.shown_native_screen()?;
        let menu = self.native_screen_menu()?;
        mod_api::RecipeBookLayout::registered(menu)?;
        Some(horizon_client::recipe_book::placement_sync::Window {
            window: screen.window,
            nonce: screen.layout_revision,
            menu: menu.to_owned().try_into().ok()?,
            book: screen
                .screen
                .recipe_book
                .as_ref()?
                .as_str()
                .to_owned()
                .try_into()
                .ok()?,
        })
    }
}
