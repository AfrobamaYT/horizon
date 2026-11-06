//! The game's side of the `dragon_refs` area (`horizon-mod-api/src/host/dragon_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl DragonRefsServerHost for Game<'_> {
    fn dragon(&self, entity: EntityRef) -> Option<DragonRef> {
        mod_api::__private::check_access("dragon", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dragon_ref(entity)
    }
}
impl DragonRefsBackend for Game<'_> {
    fn dragon_phase(&self, dragon: DragonRef) -> Option<NativeDragonPhase> {
        mod_api::__private::check_access("dragon_phase", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dragon_phase(dragon)
    }
    fn dragon_alive_crystals(&self, dragon: DragonRef) -> Option<i32> {
        mod_api::__private::check_access(
            "dragon_alive_crystals",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_alive_crystals(dragon)
    }
    fn dragon_previously_killed(&self, dragon: DragonRef) -> Option<bool> {
        mod_api::__private::check_access(
            "dragon_previously_killed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_previously_killed(dragon)
    }
}
impl DragonRefsBackendMut for Game<'_> {
    fn dragon_set_phase(&mut self, dragon: DragonRef, phase: NativeDragonPhase) -> bool {
        mod_api::__private::check_access(
            "dragon_set_phase",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_set_phase(dragon, phase)
    }
}
