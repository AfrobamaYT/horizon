//! The `register_range_select_item_model_property` hooks: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The numeric item model properties a mod adds (NeoForge's
    /// `RegisterRangeSelectItemModelPropertyEvent`): push each id, `<mod id>:<path>`. Asked once at
    /// start on the client, before resources load. A `minecraft:range_dispatch` naming a registered
    /// property thresholds on the number `item_model_range_property` answers per stack, also after a
    /// resource reload. An id in vanilla's namespace, not of that form, or registered already is
    /// logged and left out; a property nobody registers reads 0.
    #[hook(
        side = "client",
        contract = "collect",
        out = "properties",
        context = "registration",
        neoforge = "RegisterRangeSelectItemModelPropertyEvent"
    )]
    fn register_range_select_item_model_property();
}
