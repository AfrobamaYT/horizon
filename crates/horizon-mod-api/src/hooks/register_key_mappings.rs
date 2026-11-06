//! The `register_key_mappings` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The keys native mods register on Options > Controls (NeoForge's `RegisterKeyMappingsEvent`):
    /// push yours. Asked when the mod list is made, so a mod's keys are listed once the mod is on
    /// Options > Mods. Each key is listed under its mod's name, its category and its label, and the
    /// player's binding and reset are kept by mod id in `horizon-mods.txt`. Client-side only.
    #[hook(
        side = "client",
        contract = "collect",
        out = "mappings",
        neoforge = "RegisterKeyMappingsEvent"
    )]
    fn register_key_mappings();
}
