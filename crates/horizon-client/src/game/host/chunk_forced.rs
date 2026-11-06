//! The game's side of the `chunk_forced` area (`horizon-mod-api/src/host/chunk_forced.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ChunkForcedBackend for Game<'_> {
    fn level_chunk_ref(&self, level: LevelRef, pos: [i32; 2]) -> Option<ChunkRef> {
        mod_api::__private::check_access(
            "level_chunk_ref",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_chunk_ref(level, pos)
    }
    fn chunk_forced(&self, chunk: ChunkRef) -> Option<bool> {
        mod_api::__private::check_access("chunk_forced", mod_api::__private::LoadAccess::WorldOnly);
        self.native_chunk_forced(chunk)
    }
    fn level_forced_chunks(&self, level: LevelRef) -> Option<Vec<[i32; 2]>> {
        mod_api::__private::check_access(
            "level_forced_chunks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_forced_chunks(level)
    }
    fn level_mod_forced_chunks(
        &self,
        level: LevelRef,
        controller: TicketController,
    ) -> Option<Vec<(NativeTicketOwner, [i32; 2], bool)>> {
        mod_api::__private::check_access(
            "level_mod_forced_chunks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_mod_forced_chunks(level, controller)
    }
}
impl ChunkForcedBackendMut for Game<'_> {
    fn chunk_set_forced(&mut self, chunk: ChunkRef, forced: bool) -> bool {
        mod_api::__private::check_access(
            "chunk_set_forced",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_chunk_set_forced(chunk, forced)
    }
    fn chunk_set_mod_forced(
        &mut self,
        chunk: ChunkRef,
        controller: TicketController,
        owner: NativeTicketOwner,
        forced: bool,
        natural_spawning: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "chunk_set_mod_forced",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_chunk_set_mod_forced(chunk, controller, owner, forced, natural_spawning)
    }
}
