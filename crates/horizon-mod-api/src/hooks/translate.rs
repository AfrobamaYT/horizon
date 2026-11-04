//! The `translate` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn translate();
}
