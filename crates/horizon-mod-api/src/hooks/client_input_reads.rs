//! The `client_input_reads` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn playing();
    #[helper(side = "client")]
    fn mapping();
    #[helper(side = "client")]
    fn modifiers();
}
