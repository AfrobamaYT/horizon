//! The game's side of the `structure_template` area (`horizon-mod-api/src/host/structure_template.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl StructureTemplateServerHost for Game<'_> {
    fn structure_template(
        &mut self,
        id: &str,
        at: [i32; 3],
        rotation: StructureRotation,
        mirror: StructureMirror,
    ) -> Option<StructureTemplateSnapshot> {
        mod_api::__private::check_access(
            "structure_template",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_structure_template(id, at, rotation, mirror)
    }
}
