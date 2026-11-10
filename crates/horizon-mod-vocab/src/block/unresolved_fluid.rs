//! Opaque identities for fluid names retained from unavailable content.
use crate::{FluidKey, Host, UnresolvedFluid};
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

/// A registered fluid, or an opaque retained name from unavailable content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum FluidIdentity {
    /// A registered fluid.
    Resolved(FluidKey),
    /// A name retained verbatim until its declaring mod returns.
    Unresolved(UnresolvedFluid),
}

impl FluidIdentity {
    /// Its registered key, or none when the fluid is unresolved.
    pub fn key(self) -> Option<FluidKey> {
        match self {
            Self::Resolved(key) => Some(key),
            Self::Unresolved(_) => None,
        }
    }
}

impl UnresolvedFluid {
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn from_slot(slot: u64) -> Result<Self, &'static str> {
        let table = TABLE.read().unwrap();
        (slot < table.names.len() as u64)
            .then_some(Self(slot))
            .ok_or("unknown unresolved fluid")
    }

    /// The original stored name, read from the host's process-lifetime table.
    pub fn id(self, host: &dyn Host) -> String {
        host.unresolved_fluid_id(self)
    }
}

/// Compare an identity with an NBT name without allocating.
pub fn matches_utf16(identity: FluidIdentity, id: &[u16]) -> bool {
    match identity {
        FluidIdentity::Resolved(key) => {
            crate::content::registry_keys::fluid_key_utf16(id) == Some(key)
        }
        FluidIdentity::Unresolved(handle) => lookup_utf16(id) == Some(handle),
    }
}

/// Resolve an identity at an engine-owned boundary.
pub fn identity_name(identity: FluidIdentity) -> String {
    match identity {
        FluidIdentity::Resolved(key) => crate::content::registry_keys::fluid_id(key)
            .expect("resolved fluid key")
            .to_owned(),
        FluidIdentity::Unresolved(handle) => name(handle),
    }
}

