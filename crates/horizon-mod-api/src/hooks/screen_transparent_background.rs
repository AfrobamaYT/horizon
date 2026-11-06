//! Declarations carried from upstream `screen_transparent_background`; the generator owns the typed bindings.

pub trait Hooks {
    /// `Screen.renderTransparentBackground`: draws the translucent dark background
    /// behind an in-game screen (inventory, container, creative, book, sign, editor).
    /// Set `canceled` (or return `Flow::Cancel`) to suppress the vanilla dark gradient,
    /// allowing a mod to draw its own background or apply menu background blur
    /// via `Host::apply_screen_blur`. API @API:screen_transparent_background@.
    #[notes = r#"## Transparent screen backgrounds and landing changes

`Hooks::screen_transparent_background` runs immediately before an in-game screen's
translucent gradient. Returning `Flow::Cancel` suppresses that gradient, not the
screen's contents. Its `batch` is a `DrawContext`: custom drawing survives
cancellation. `ClientHost::apply_screen_blur()` requests the normal menu blur
at most once per frame; `menu_blur_radius()` reads the configured radius.
`DrawContext::draw_menu_background` and `tiled_sprite` draw menu textures through
the same atlas. The whole-screen `screen_render_pre` remains a separate moment.

`Hooks::entity_change_block_pre` can cancel a player's landing block change before
its neighbour notifications and level events. The resident write is restored
before those consumers run, so a crop above cancelled farmland stays untouched.
The event identifies the player, position and old/new block names. This veto is
separate from `entity_change_block`, the existing post-change mob notification,
and from `farmland_trample`, the farmland-specific in-write event."#]
    #[hook(
        side = "client",
        contract = "cancel",
        neoforge = "ScreenEvent.Render.Pre (transparent background)",
        cancellable = true,
        out = "canceled",
        target = "horizon_client::game::native_hooks::Game<'_>::native_screen_transparent_background"
    )]
    fn screen_transparent_background();
}

pub trait Helpers {
    /// Requests vanilla's menu background blur for this frame, with a per-frame guard
    /// so blur is applied at most once per frame. Returns true if blur was newly
    /// requested this frame, or false if it was already applied/requested this frame.
    /// API @API:screen_transparent_background@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_hooks::Game<'_>::native_apply_screen_blur"
    )]
    fn apply_screen_blur();
    /// The user's configured menu background blur radius (`menuBackgroundBlurriness` in options.txt,
    /// usually 0.0 to 10.0; default 5.0). 0.0 means blur is turned off. API @API:screen_transparent_background@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_hooks::Game<'_>::native_menu_blur_radius"
    )]
    fn menu_blur_radius();
    /// `Screen.renderMenuBackground`: draws the tiled menu background (32x32) across `rect`
    /// (`[x, y, w, h]` in GUI pixels). If `in_world` is true, uses the dark translucent
    /// texture (`gui/inworld_menu_background`); otherwise uses the opaque dirt-like
    /// texture (`gui/menu_background`). API @API:screen_transparent_background@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_hooks::Game<'_>::native_draw_menu_background"
    )]
    fn draw_menu_background();
    /// Repeats sprite `sprite` across `rect` (`[x, y, w, h]` in GUI pixels) with each tile
    /// sized `size` GUI pixels square, tinted by `argb`. False when the sprite is not in the HUD atlas.
    /// API @API:screen_transparent_background@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_hooks::Game<'_>::native_tiled_sprite"
    )]
    fn tiled_sprite();
}
