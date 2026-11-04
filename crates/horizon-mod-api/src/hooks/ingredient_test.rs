//! The `ingredient_test` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A mod's own ingredient kind, matching a concrete stack at craft time:
    /// the ingredient's `test` calls here with the kind, its JSON and the
    /// stack being matched, `accept` already true (the load-time list named
    /// the item). Set `accept` to false to refuse the stack — its components
    /// are the usual reason. The hook may be called many times a craft; the
    /// answer must be the same for the same stack.
    #[hook(
        side = "both",
        contract = "listen",
        target = "mod_api::crafting::native_ingredient_test"
    )]
    fn ingredient_test();
}
