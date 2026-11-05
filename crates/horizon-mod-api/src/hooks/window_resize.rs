//! Window resize hook declaration.

pub trait Hooks {
    /// The client window's physical size changed, after handling the resize.
    /// Repeated notifications of the same size are not dispatched.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "WindowResizeEvent",
        cancellable = false
    )]
    fn window_resize();
}
