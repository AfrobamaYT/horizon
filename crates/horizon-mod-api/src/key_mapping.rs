//! The keys a native mod registers on Options > Controls.

use crate::ClientModifiers;

/// A key a native mod registers (`register_key_mappings`): it is listed under the mod's name on
/// Options > Controls, and the player's binding and reset are kept by mod id in `horizon-mods.txt`.
/// Its default is `key` with `modifiers` held; shift, control and alt are bound, super is not.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NativeKeyRegistration {
    /// The id of the mod that registers it: the heading on Controls and the key of its saved binding.
    pub mod_id: String,
    /// The key's id, unique within its mod (`zoom`).
    pub id: String,
    /// The label Controls shows (a literal).
    pub label: String,
    /// The default key (`key.keyboard.z`).
    pub key: String,
    /// The default modifiers, in GLFW's input-event encoding (shift 1, control 2, alt 4).
    pub modifiers: ClientModifiers,
    /// The category it is listed under within its mod's heading; empty for none.
    pub category: String,
}

impl NativeKeyRegistration {
    /// Key `id` of mod `mod_id`, bound to `key` with no modifiers and labelled `id`.
    pub fn new(mod_id: &str, id: &str, key: &str) -> Self {
        Self {
            mod_id: mod_id.to_owned(),
            id: id.to_owned(),
            label: id.to_owned(),
            key: key.to_owned(),
            modifiers: ClientModifiers::default(),
            category: String::new(),
        }
    }

    /// Sets the label Controls shows.
    pub fn label(mut self, label: &str) -> Self {
        self.label = label.to_owned();
        self
    }

    /// Sets the modifiers the default binding holds.
    pub fn modifiers(mut self, modifiers: ClientModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets the category the key is listed under.
    pub fn category(mut self, category: &str) -> Self {
        self.category = category.to_owned();
        self
    }
}
