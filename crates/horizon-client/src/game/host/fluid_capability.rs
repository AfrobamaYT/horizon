//! The game's side of the `fluid_capability` area (`horizon-mod-api/src/host/fluid_capability.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl FluidCapabilityBackend for Game<'_> {
    fn fluid_tank_count(&self, at: BlockSide) -> Option<usize> {
        mod_api::__private::check_access(
            "fluid_tank_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_count(at)
    }
    fn fluid_tank(&self, at: BlockSide, tank: usize) -> Option<FluidStack> {
        mod_api::__private::check_access(
            "fluid_tank",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank(at, tank)
    }
    fn fluid_tank_capacity(
        &self,
        at: BlockSide,
        tank: usize,
        fluid: &FluidStack,
    ) -> u32 {
        mod_api::__private::check_access(
            "fluid_tank_capacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_capacity(at, tank, fluid)
    }
}
impl FluidCapabilityBackendMut for Game<'_> {
    fn fluid_tank_insert(
        &mut self,
        at: BlockSide,
        tank: usize,
        fluid: FluidStack,
        tx: &Transaction,
    ) -> Option<FluidStack> {
        mod_api::__private::check_access(
            "fluid_tank_insert",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_insert(at, tank, fluid, tx)
    }
    fn fluid_tank_extract(
        &mut self,
        at: BlockSide,
        tank: usize,
        max: u32,
        tx: &Transaction,
    ) -> Option<FluidStack> {
        mod_api::__private::check_access(
            "fluid_tank_extract",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_extract(at, tank, max, tx)
    }
}
