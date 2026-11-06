//! Tooltip helpers carried from upstream, with drawing owned by `DrawContext` and modifiers by `ClientHost::modifiers`.

pub trait Helpers {
    /// Draw an item stack with custom count/decoration text (or None) at `at` in GUI units.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_client_services::Game<'_>::native_gui_draw_item_with"
    )]
    fn gui_draw_item_with();
    /// Whether a specific key is currently held down.
    /// `key` is the vanilla physical key name (`key.keyboard.left.shift`, for example).
    /// Unknown names are not down. Modifier combinations use `ClientHost::modifiers`.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_client_services::Game<'_>::native_is_key_down"
    )]
    fn is_key_down();
}
