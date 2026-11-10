//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
/// A native mod's book category for one custom recipe type.
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct RecipeBookCategory {
    /// A namespaced category id outside `minecraft:`.
    pub id: String,
    /// A custom type present in the current recipe table.
    pub recipe_type: String,
    /// A registered item id used as the category's icon.
    pub icon: String,
}

/// A namespaced custom recipe book identity returned during registration.
/// This owns no settings; player settings and synchronization are separate.
// exhaustive: its field is private, so mods cannot build or destructure it
#[derive(Clone, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
pub struct RecipeBookTypeKey {
    pub(crate) id: String,
}

impl RecipeBookTypeKey {
    /// Whether this process has installed this identity in its frozen registry.
    pub fn is_registered(&self) -> bool {
        crate::parts::installed()
            .is_some_and(|registry| registry.recipe_book_types.contains_key(self.as_str()))
    }

    /// The stable namespaced identity used in saves and negotiated payloads.
    pub fn as_str(&self) -> &str {
        &self.id
    }
}
