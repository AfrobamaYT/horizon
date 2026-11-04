//! Shared item-data producer for tests and standalone probes, not a runtime API.

use std::{collections::BTreeMap, path::Path, sync::Arc};

use horizon_world::{
    item_defaults::ItemData,
    native_load_context::{DataLoadOrigin, LoadContext, LoadInputs},
};

pub(crate) fn item_data(
    root: &Path,
    origin: DataLoadOrigin,
) -> horizon_client_core::Result<Arc<ItemData>> {
    let inputs = LoadInputs::new(BTreeMap::new(), root.join("settings"));
    Ok(Arc::new(ItemData::load(
        root,
        LoadContext::new(origin, Arc::new(inputs), []),
    )?))
}

/// A server's data load with no mods and no settings, for loaders under test.
#[allow(
    dead_code,
    reason = "standalone probes that include this file may not load recipes"
)]
#[allow(
    dead_code,
    reason = "standalone probes that include this file may not load recipes"
)]
/// The recipes of a server load over `root`, resolved once as a load does.
pub(crate) fn server_sources(
    root: &std::path::Path,
    overlay: &horizon_world::loot::PackOverlay,
) -> horizon_client::crafting::ResolvedSources {
    horizon_client::crafting::ResolvedSources::load_or_empty(root, overlay, &server_load())
}

pub(crate) fn server_load() -> LoadContext {
    LoadContext::new(
        DataLoadOrigin::ServerDataLoad,
        Arc::new(LoadInputs::new(BTreeMap::new(), std::path::PathBuf::new())),
        [],
    )
}
