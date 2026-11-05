//! The `gather_effect_screen_tooltips` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `GatherEffectScreenTooltipsEvent`: the tooltip of the effect panel the
    /// cursor is over beside the survival or creative inventory, shown when
    /// the panel is compact or its name is cut short. `tooltip` holds the
    /// effect's full name then its duration, each with a colour as `rgba`
    /// packs it (`u32::MAX` keeps the text's own); add, change, reorder or
    /// remove lines, and an empty list draws nothing. `effect` is the effect
    /// the panel shows, `screen` the inventory. Asked each frame the tooltip
    /// shows, only while a mod listens. Client-side.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "GatherEffectScreenTooltipsEvent",
        cancellable = false
    )]
    fn gather_effect_screen_tooltips();
}
