use super::*;
use crate::world::datapack_registry::{
    DatapackField, DatapackRegistrationError, DatapackRegistryDeclaration, DatapackRegistrySync,
    check_schema, valid_namespaced,
};

/// The datapack registries mods declared.
pub fn datapack_registry_declarations() -> &'static [DatapackRegistryDeclaration] {
    installed().map_or(&[], |installed| &installed.datapack_registries)
}

/// Registration of mod-owned datapack registries
/// ([`Registrar::declare_datapack_registry`](DatapackRegistriesRegistrar::declare_datapack_registry)).
pub trait DatapackRegistriesRegistrar {
    /// Declare a datapack registry of the mod's own (NeoForge's
    /// `DataPackRegistryEvent.NewRegistry`): `path` in the mod's namespace
    /// (`spell` of `example` is `example:spell`), the flat record `fields` an
    /// entry has, and how clients receive the entries.
    ///
    /// Entries are JSON files in data packs: entry `addon:fire` of
    /// `example:spell` is `data/addon/example/spell/fire.json`
    /// ([`DatapackRegistryDeclaration::entry_file`]). The game decodes every
    /// file against `fields`: no mod code runs while decoding, so native and
    /// wasm mods get the same bounds. A key the schema does not
    /// declare, a missing required key or an out-of-range value fails the
    /// entry; a default never stands in for invalid data.
    ///
    /// Only both-side packages may declare datapack registries. Refused,
    /// naming the mod: a key outside the mod's namespace (a vanilla
    /// registry's included), a malformed key, a duplicate (of this mod's or
    /// another's, caught where the declarations join), an empty or repeated
    /// field name, unusable bounds, or a default that is not a value of its
    /// field inside its bounds.
    fn declare_datapack_registry(
        &mut self,
        path: &str,
        sync: DatapackRegistrySync,
        fields: Vec<DatapackField>,
    ) -> Result<(), DatapackRegistrationError>;
}





#[cfg(test)]
mod default_tests {
    use super::*;
    #[test]
    fn registered_identifier_defaults_match_explicit_values() {
        for kind in [
            crate::DatapackFieldKind::Identifier,
            crate::DatapackFieldKind::Reference {
                registry: "minecraft:item".into(),
            },
        ] {
            let mut registrar = PartRegistrar::new("defaults");
            registrar
                .declare_datapack_registry(
                    "entry",
                    DatapackRegistrySync::default(),
                    vec![DatapackField::with_default(
                        "id",
                        kind,
                        crate::DatapackValue::Text("ice".into()),
                    )],
                )
                .unwrap();
            let declaration = &registrar.datapack_registries[0];
            assert_eq!(
                declaration.decode_entry("{}").unwrap(),
                declaration.decode_entry(r#"{"id":"ice"}"#).unwrap()
            );
        }
    }
}
