//! Client window focus and observed screen reads.

pub trait Helpers {
    /// Whether the client window is focused, as last reported by the window system.
    #[notes = r#"## Client window and screen state

`ClientHost::window_focused()` reports the last focus state from the window
system. `ClientHost::observed_screen_id()` returns the same ID screen hooks see,
including overlays. `None` means no view is available: there may be no open
screen or no renderer yet. It does not mean the client is outside a world.
Read again after a screen or focus change."#]
    #[helper(side = "client")]
    fn window_focused();
    /// The screen ID seen by screen hooks, including overlays. None means no view
    /// is available, including when no screen or renderer exists; it does not
    /// imply that the client is outside a world.
    #[helper(side = "client")]
    fn observed_screen_id();
}
