//! The `enchantments` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn enchantments();
    #[helper(side = "client")]
    fn enchantment_min_cost();
    #[helper(side = "client")]
    fn enchantment_supports();
}
