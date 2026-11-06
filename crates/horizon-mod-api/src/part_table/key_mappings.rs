/// Owner-bound declarations retained for client mod-list rebuilds.
pub fn key_mappings() -> &'static [(&'static str, super::NativeKeyRegistration)] {
    installed().map_or(&[], |installed| &installed.keys.key_mappings)
}

/// What a mod's [`Hooks::register`](super::Hooks::register) attaches its parts with, and
/// declares its block entity types and ticket controllers on. A thing takes one part of each
/// kind: the same kind twice on one thing refuses the mods at startup, naming both.
///
/// Mods call the generic methods on `dyn Registrar` after requesting registry keys;
/// they reach the trait's own methods, one implementation for native and wasm mods.
pub trait Registrar {
    /// Register a Controls binding owned by the calling mod (`RegisterKeyMappingsEvent`).
    /// Called from `Hooks::register`; invalid or duplicate local ids refuse startup.
    fn register_key_mapping(&mut self, mapping: super::NativeKeyRegistration);
}

impl Registrar for PartRegistrar {
    fn register_key_mapping(&mut self, mapping: super::NativeKeyRegistration) {
        if mapping.id.is_empty()
            || !mapping
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
            || self
                .parts
                .key_mappings
                .iter()
                .any(|(owner, prior)| *owner == self.mod_id && prior.id == mapping.id)
        {
            self.refused.push(format!(
                "mod {}: duplicate or invalid key mapping {:?}",
                self.mod_id, mapping.id
            ));
            return;
        }
        self.parts.key_mappings.push((self.mod_id, mapping));
    }
}
/// Fixtures that tests of other crates use to install Parts.
#[cfg(any(test, feature = "testing"))]
pub mod tests {

    #[test]
    fn key_mappings_are_owned_and_duplicates_refuse_startup() {
        let mut registrar = super::PartRegistrar::new("first");
        registrar.register_key_mapping(NativeKeyRegistration::new("zoom", "key.keyboard.z"));
        registrar.register_key_mapping(NativeKeyRegistration::new("zoom", "key.keyboard.r"));
        registrar.register_key_mapping(NativeKeyRegistration::new("second:zoom", "key.keyboard.z"));
        registrar.mod_id = "second";
        registrar.register_key_mapping(NativeKeyRegistration::new("zoom", "key.keyboard.z"));
        assert_eq!(registrar.refused.len(), 2);
        let owners: Vec<_> = registrar
            .parts
            .key_mappings
            .iter()
            .map(|(owner, _)| *owner)
            .collect();
        assert_eq!(owners, ["first", "second"]);
        assert_eq!(registrar.parts.key_mappings[0].1.key, "key.keyboard.z");
    }
}
