//! The game's side of the `energy_capability` area (`horizon-mod-api/src/host/energy_capability.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl EnergyCapabilityBackend for Game<'_> {
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
}
impl EnergyCapabilityBackendMut for Game<'_> {
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
}
