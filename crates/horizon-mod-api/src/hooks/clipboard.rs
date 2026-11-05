//! The `clipboard` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::clipboard::Game<'_>::native_clipboard_text"
    )]
    fn clipboard_text();
    #[helper(
        side = "client",
        target = "horizon_client::game::clipboard::Game<'_>::native_set_clipboard_text"
    )]
    fn set_clipboard_text();
}
