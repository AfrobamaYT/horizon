//! The `item_model_update` hooks: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A node of a type a mod registered (`register_item_models`, vanilla's `ItemModel.update`)
    /// is resolved for a stack, in hand, in a screen or on the ground: push the names of the
    /// node's `models` to draw, in order, only for a type your mod owns. `config` borrows the
    /// node's JSON object, converted once at load. The owned stack snapshot is the one being
    /// resolved, including nested bundle items. `bundle_selected` is the runtime bundle selection
    /// index; -1 means none.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "none (vanilla ItemModel.update of a type RegisterItemModelsEvent registers; NeoForge adds no event for it)"
    )]
    fn item_model_update();
}
