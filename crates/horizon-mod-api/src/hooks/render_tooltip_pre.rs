//! The `render_tooltip_pre` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `RenderTooltipEvent.Pre`: a tooltip, laid out and about to be placed and drawn. `place`
    /// holds the point it hangs from in GUI pixels (the cursor, or the point the screen gives
    /// it), before the tooltip positioner adds its offset and keeps the box on screen, and the
    /// screen's size; set `x` and `y` to move it. `elements` are its text lines, then its images, as `render_tooltip_gather_components`
    /// left them (a tooltip with no item has only lines), and are read-only here. `stack` is the
    /// item the tooltip describes, `None` for a text tooltip. Set `cancel` and nothing is drawn,
    /// and no `tooltip_color` is asked. Asked each frame a tooltip shows, only while a mod listens.
    /// Client-side.
    #[hook(
        side = "client",
        contract = "cancel",
        out = "cancel",
        neoforge = "RenderTooltipEvent.Pre",
        cancellable = true
    )]
    fn render_tooltip_pre();
}
