//! Resource names derived from the identity resolved by the client's profession table.
use super::{Cow, interned};
use crate::mobs::villager::{self, ProfessionState};

pub(super) fn profession_name(state: &ProfessionState) -> &str {
    match state {
        ProfessionState::Registered(id) => id,
        _ => villager::profession_id(state.index().unwrap_or(0)),
    }
}

/// Resolve the namespace of a profession to the shared atlas's sprite name.
pub(super) fn profession_texture(folder: &str, id: &str) -> Cow<'static, str> {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    if namespace == "minecraft" {
        interned(format_args!("{folder}/profession/{path}"))
    } else {
        interned(format_args!("{namespace}:{folder}/profession/{path}"))
    }
}
