//! The game's side of the `tag_fluid_reads` area (`horizon-mod-api/src/host/tag_fluid_reads.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl TagFluidReadsHost for Game<'_> {
    fn unresolved_fluid_id(&self, fluid: UnresolvedFluid) -> String {
        mod_api::__private::check_access(
            "unresolved_fluid_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_unresolved_fluid_id(fluid)
    }
    fn block_in_tag(&self, block: BlockKey, tag: &str) -> Option<bool> {
        mod_api::__private::check_access(
            "block_in_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_in_tag(block, tag)
    }
    fn tag_members(&self, registry: TagRegistry, tag: &str) -> Option<Vec<String>> {
        mod_api::__private::check_access(
            "tag_members",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_tag_members(registry, tag)
    }
    fn state_outline(&self, state: BlockStateId) -> Option<Vec<[f64; 6]>> {
        mod_api::__private::check_access(
            "state_outline",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_state_outline(state)
    }
    fn state_fluid_state(&self, state: BlockStateId) -> Option<FluidState> {
        mod_api::__private::check_access(
            "state_fluid_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_state_fluid_state(state)
    }
    fn fluid_in_tag(&self, fluid: &str, tag: &str) -> Option<bool> {
        mod_api::__private::check_access(
            "fluid_in_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_in_tag(fluid, tag)
    }
    fn fluid_is_same(&self, fluid: &str, other: &str) -> bool {
        mod_api::__private::check_access(
            "fluid_is_same",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_is_same(fluid, other)
    }
    fn fluid_bucket(&self, fluid: &str) -> Option<&'static str> {
        mod_api::__private::check_access(
            "fluid_bucket",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_bucket(fluid)
    }
    fn fluid_pickup_sound(&self, fluid: &str) -> Option<&'static str> {
        mod_api::__private::check_access(
            "fluid_pickup_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_pickup_sound(fluid)
    }
}
impl TagFluidReadsBackend for Game<'_> {
    fn level_fluid_state(&self, level: LevelRef, pos: [i32; 3]) -> Option<FluidState> {
        mod_api::__private::check_access(
            "level_fluid_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_fluid_state(level, pos)
    }
}
