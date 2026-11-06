//! The game's side of the `fluid_draw` area (`horizon-mod-api/src/host/fluid_draw.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl FluidDrawClientHost for Game<'_> {
    fn fluid_name(&self, fluid: &str) -> String {
        mod_api::__private::check_access(
            "fluid_name",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_fluid_name(fluid)
    }
}
