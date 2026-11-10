//! Owned authoring and validation; lookup storage stays below the engine.
include!(concat!(env!("OUT_DIR"), "/parts_registration_modules.rs"));
impl Registrar for PartRegistrar {}
