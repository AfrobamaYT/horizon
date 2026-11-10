//! The `render_hand` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0098

pub trait Hooks {
    /// Before one first-person hand, item, map or empty arm is drawn
    /// (`RenderHandEvent`, client). Adjust `render`'s pose or cancel the
    /// default geometry. Equip progress includes the item's swap scale.
    /// Unlike NeoForge, this does not expose the full pose stack or player
    /// render state; to draw in the hand, use `native_hand_submit`.
    #[hook(
        side = "client",
        contract = "cancel",
        out = "canceled",
        neoforge = "RenderHandEvent",
        cancellable = true
    )]
    fn render_hand();
}
