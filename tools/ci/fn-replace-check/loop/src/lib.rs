//! Two `[[native.loop]]` slots in `block_entities.tick` (`src/loops/`), compiled into the launcher's
//! copy of the loop, and a HEAD mixin on the loop's entry (`src/mixins/`).
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

/// Called by the helpers: a helper may use its mod's crate.
pub fn note(what: &str) {
    eprintln!("fnr: loop {what}");
}
