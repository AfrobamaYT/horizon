//! The `gui_render_post` declarations reconciled with the fork API.

pub trait Hooks {
    /// After the complete normal GUI batch, including screens, prompts, toasts,
    /// debug overlays, the pad cursor and IME. Fires even with F1 or a menu open;
    /// file/world loading and resource-reload overlays do not fire it. The batch
    /// is in GUI pixels. `native_camera` is this frame's world camera (None on
    /// the panorama). `gui_hidden` is the F1 setting; `focused` is window focus.
    /// A mod decides whether its foreground indicator respects those flags.
    /// Client only; the host is read-only. API @API:gui_render_post@.
    #[hook(
        side = "client",
        contract = "listen",
        cancellable = false,
        neoforge = "RenderGuiEvent.Post"
    )]
    fn gui_render_post();
}
