//! The `render_item_in_frame` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Before a visible item frame's item or map is drawn. Cancel hides the item, not the frame.
    #[hook(
        side = "client",
        contract = "cancel",
        out = "canceled",
        neoforge = "RenderItemInFrameEvent",
        cancellable = true
    )]
    fn render_item_in_frame();
}
