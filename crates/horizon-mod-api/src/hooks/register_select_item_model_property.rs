//! The `register_select_item_model_property` hooks: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The categorical item model properties a mod adds (NeoForge's
    /// `RegisterSelectItemModelPropertyEvent`): push each id, `<mod id>:<path>`. Asked once at start
    /// on the client, before resources load. A `minecraft:select` naming a registered property picks
    /// the case whose `when` string equals what `item_model_select_property` answers per stack, also
    /// after a resource reload. An id in vanilla's namespace, not of that form, or registered already
    /// is logged and left out; a select on a property nobody registers draws its fallback.
    #[hook(
        side = "client",
        contract = "collect",
        out = "properties",
        context = "registration",
        neoforge = "RegisterSelectItemModelPropertyEvent"
    )]
    fn register_select_item_model_property();
}
