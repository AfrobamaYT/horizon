//! The `item_model_property` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Client-side evaluation of a mod's namespaced conditional item model property.
    /// Set value only for a property your mod owns; unhandled properties stay false.
    /// The borrowed stack is the one being resolved, including nested bundle items.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "RegisterConditionalItemModelPropertyEvent"
    )]
    fn item_model_property();
}
