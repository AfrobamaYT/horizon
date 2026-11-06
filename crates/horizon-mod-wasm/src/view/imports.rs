//! WIT-derived synchronous registrations retain the store for fuel charging.
use super::*;
use wasmtime::{AsContextMut, component::ResourceType};

pub(super) fn add_to_linker(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    linker
        .instance("horizon:mod-internals/view@0.1.0")?
        .resource("node", ResourceType::host::<wire::Node>(), |store, rep| {
            Metered::drop(store, Resource::new_own(rep))
        })?;
    include!(concat!(env!("OUT_DIR"), "/internals_imports.rs"));
    Ok(())
}
