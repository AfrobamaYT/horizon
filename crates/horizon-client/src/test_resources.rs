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
