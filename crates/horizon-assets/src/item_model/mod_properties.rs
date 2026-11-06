//! The item model properties native mods register (NeoForge's
//! `RegisterRangeSelectItemModelPropertyEvent`). A `range_dispatch` on a registered
//! property reads its number from the host per stack; one nobody registered stays
//! a foreign property and reads 0 (`foreign.rs`).
use super::identifier;
use std::{collections::BTreeSet, sync::RwLock};

static RANGE: RwLock<BTreeSet<String>> = RwLock::new(BTreeSet::new());

/// Keeps the numeric `range_dispatch` properties native mods registered, before
/// resources load. An id that is not `<namespace>:<path>` in a namespace other than
/// vanilla's is logged and left out. Returns the ids kept, in registration order.
pub fn register_range_properties(ids: Vec<String>) -> Vec<String> {
    let mut kept = Vec::new();
    let mut set = RANGE.write().unwrap_or_else(|e| e.into_inner());
    for id in ids {
        let own = identifier(&id).is_ok_and(|parsed| parsed == id) && !id.starts_with("minecraft:");
        if !own {
            log::warn!(
                "range select item model property {id:?} is not <namespace>:<path> in a mod's namespace"
            );
        } else if !set.insert(id.clone()) {
            log::warn!("range select item model property {id} is registered twice");
        } else {
            kept.push(id);
        }
    }
    kept
}

/// Whether a native mod registered `id` as a numeric `range_dispatch` property.
pub fn is_range_property(id: &str) -> bool {
    RANGE.read().unwrap_or_else(|e| e.into_inner()).contains(id)
}
