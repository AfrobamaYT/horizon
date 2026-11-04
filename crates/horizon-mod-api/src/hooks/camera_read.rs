//! The `camera_read` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn camera();
    #[helper(side = "both")]
    fn block_collision();
}
