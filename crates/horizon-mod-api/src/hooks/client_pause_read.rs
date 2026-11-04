//! The `client_pause_read` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn is_paused();
}
