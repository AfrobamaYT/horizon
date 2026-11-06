//! The game's side of the `level_neighbor_updates` area (`horizon-mod-api/src/host/level_neighbor_updates.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelNeighborUpdatesBackendMut for Game<'_> {
    fn level_update_neighbors(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
        update: NeighbourUpdate,
    ) -> bool {
        mod_api::__private::check_access(
            "level_update_neighbors",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_update_neighbors(level, pos, source, update)
    }
    fn level_neighbor_changed(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
    ) -> bool {
        mod_api::__private::check_access(
            "level_neighbor_changed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_neighbor_changed(level, pos, source)
    }
}
