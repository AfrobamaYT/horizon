//! The game's side of the `world_edit` area (`horizon-mod-api/src/host/world_edit.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl WorldEditServerHost for Game<'_> {
    fn edit_blocks(&mut self, dimension: &str, edit: &NativeBlockEdit) -> Result<u64, String> {
        mod_api::__private::check_access("edit_blocks", mod_api::__private::LoadAccess::WorldOnly);
        self.native_edit_blocks(dimension, edit)
    }
    fn surface_heights(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
    ) -> Result<Vec<Option<i32>>, String> {
        mod_api::__private::check_access(
            "surface_heights",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_surface_heights(dimension, columns, heightmap)
    }
    fn read_columns(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
        range: (u32, u32),
    ) -> Result<NativeColumnBlocks, String> {
        mod_api::__private::check_access("read_columns", mod_api::__private::LoadAccess::WorldOnly);
        self.native_read_columns(dimension, columns, heightmap, range)
    }
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
