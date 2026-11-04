//! The `tooltip_color` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Change a tooltip box's background and border gradient colors, as ARGB.
    /// Stack is None for tooltips not associated with an item.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "RenderTooltipEvent.Texture"
    )]
    fn tooltip_color();
}
