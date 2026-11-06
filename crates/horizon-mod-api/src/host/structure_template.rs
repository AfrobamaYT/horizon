//! The `structure_template` area: host methods the game implements (`game/host/structure_template.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `structure_template` area, implemented by the game and reached through [`ServerHost`].
pub trait StructureTemplateServerHost {
    /// Side: server.
    /// For a server mod: structure template `id` (`mymod:hut`, found as
    /// `StructureTemplateManager.get` finds it: the world's saved structures,
    /// then the game's and the mods' own) as `/place template id` would place
    /// it at `at` with `rotation` and `mirror`, without placing anything. The
    /// blocks come in vanilla's placement order (full blocks, the others, then
    /// those with a block entity, each by y, x, z), from the palette
    /// `/place template` picks at `at`; no processor runs, and the template's
    /// entities are left out. None for an id that doesn't parse or has a `.`
    /// or `..` part, a template the game doesn't have, and on a client.
    /// API @API:structure_template@.
    fn structure_template(
        &mut self,
        id: &str,
        at: [i32; 3],
        rotation: StructureRotation,
        mirror: StructureMirror,
    ) -> Option<StructureTemplateSnapshot> {
        crate::phase::unprovided(
            "ServerHost::structure_template",
            "structure_template",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
