use super::*;
/// Owner-bound declarations retained for client mod-list rebuilds.
pub fn key_mappings() -> &'static [(&'static str, crate::NativeKeyRegistration)] {
    installed().map_or(&[], |installed| &installed.keys.key_mappings)
}

/// Fixtures that tests of other crates use to install Parts.
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    use super::*;
    use crate::*;

    #[test]
    pub(crate) fn key_mappings_are_owned_and_duplicates_refuse_startup() {
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
