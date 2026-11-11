//! The `light_emission` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The light the block `state` emits at `pos` in `dimension`
    /// (`IBlockExtension.getLightEmission`), for a block that declares
    /// `dynamic_light: true`; parts on other blocks are never asked. `emission`
    /// starts empty: set `Some(level)` to replace the state's own light at that
    /// position (above 15 is clamped to 15), or leave it empty to keep it. Asked
    /// on the thread that owns the level, both sides, as a chunk holding the
    /// block comes in and as an edit sets it; the light engine keeps the answer
    /// until the cell's state changes or the chunk unloads, and never asks it
    /// on its own. The level is read-only. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "replace",
        out = "emission",
        scope = "block",
        key = "state",
        host = "read",
        neoforge = "IBlockExtension.getLightEmission, IBlockStateExtension.getLightEmission"
    )]
    fn light_emission();
}
