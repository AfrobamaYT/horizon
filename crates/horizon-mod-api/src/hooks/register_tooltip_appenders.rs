//! The `register_tooltip_appenders` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `RegisterTooltipAppendersEvent`: push the id of each data component whose tooltip lines a mod adds
    /// in `tooltip_appender`, namespaced as a data component id reads (`mymod:charge`). Asked once at start,
    /// on the client, as `layer_definitions` is. An id that isn't namespaced is logged and left out; one
    /// registered twice is kept once.
    #[hook(
        side = "client",
        contract = "collect",
        out = "appenders",
        context = "registration",
        neoforge = "RegisterTooltipAppendersEvent"
    )]
    fn register_tooltip_appenders();
}
