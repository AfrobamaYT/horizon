//! The `mod_mismatch` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A world opened whose saved mod list differs from the mods loaded now
    /// (NeoForge's `ModMismatchEvent`): `mismatches` names each mod the world
    /// was saved with whose version changed, or that is no longer loaded
    /// (`current_version` none). A mod loaded now that the world wasn't saved
    /// with is no mismatch, and a version unknown on either side (`""`)
    /// compares equal. Push a mod id into `resolved` to mark its mismatch
    /// handled, as `markResolved` does: each mismatch nobody resolves is
    /// logged as a warning once the hook returns. Fired once per world open,
    /// after `level.dat` is read and before any chunk loads, only when there
    /// is at least one mismatch; nothing stops the world from opening.
    /// Server-side (the single player's integrated server included).
    #[notes = r##"## Mod mismatch

A world records the code mods it was saved with at the root of `level.dat`, beside `Data`, under
`fml.LoadingModList`, the list NeoForge keeps: one `{ModId, ModVersion}`
compound per loaded mod, in load order, rewritten on every save of the world.
The version is the mod's `horizon-mod.toml` `version`, or `""` when the game
doesn't know it. A world without the list (vanilla, or saved before Horizon
wrote one) is no mismatch: it fires nothing and gets the list on its next save.

`Hooks::mod_mismatch` runs once per world open, after `level.dat` is read and
before any chunk loads, when the list and the loaded mods differ: a saved mod
whose version is not the loaded one (both known), or one no longer loaded.
Read `event.mismatches`, migrate your own data for the versions you know how
to read, then push your mod id into `event.resolved`. Each unresolved mismatch
is logged as a warning naming the mod and both versions (`missing` for a mod
no longer loaded). Nothing blocks the open, and no screen is shown."##]
    #[hook(
        side = "server",
        contract = "collect",
        out = "resolved",
        collect_validator = "crate::game::mismatch_resolution::attribute",
        neoforge = "ModMismatchEvent"
    )]
    fn mod_mismatch();
}
