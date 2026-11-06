//! The game's side of the `fluids_energy` area (`horizon-mod-api/src/host/fluids_energy.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl FluidsEnergyBackend for Game<'_> {
    fn energy_amount(&self, at: BlockSide) -> Option<u32> {
        mod_api::__private::check_access(
            "energy_amount",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_energy_amount(at)
    }
    fn energy_capacity(&self, at: BlockSide) -> u32 {
        mod_api::__private::check_access(
            "energy_capacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_energy_capacity(at)
    }
    fn fluid_tank_count(&self, at: BlockSide) -> Option<usize> {
        mod_api::__private::check_access(
            "fluid_tank_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_count(at)
    }
    fn fluid_tank(&self, at: BlockSide, tank: usize) -> Option<FluidStack> {
        mod_api::__private::check_access("fluid_tank", mod_api::__private::LoadAccess::WorldOnly);
        self.native_fluid_tank(at, tank)
    }
    fn fluid_tank_capacity(&self, at: BlockSide, tank: usize, fluid: &FluidStack) -> u32 {
        mod_api::__private::check_access(
            "fluid_tank_capacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_tank_capacity(at, tank, fluid)
    }
}
impl FluidsEnergyBackendMut for Game<'_> {
    fn energy_insert(&mut self, at: BlockSide, amount: u32, tx: &Transaction) -> u32 {
        mod_api::__private::check_access(
            "energy_insert",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_energy_insert(at, amount, tx)
    }
    fn energy_extract(&mut self, at: BlockSide, amount: u32, tx: &Transaction) -> u32 {
        mod_api::__private::check_access(
            "energy_extract",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_energy_extract(at, amount, tx)
    }
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
