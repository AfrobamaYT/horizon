//! Game-owned dimension codec and registered pack-entry conversion.
use horizon_world::worldgen::overrides::{LevelHeight, ModDimension};
use mod_api::DimensionSettings;
use serde_json::{Value, json};

fn dimension_type(settings: &DimensionSettings) -> Value {
    json!({
        "min_y": settings.min_y,
        "height": settings.height,
        "logical_height": settings.logical_height,
        "has_skylight": settings.has_skylight,
        "has_ceiling": settings.has_ceiling,
    })
}

/// The actual engine dimension-type codec, called before registration is installed.
pub(in crate::game) fn valid_shape(settings: &DimensionSettings) -> bool {
    LevelHeight::of_json(&dimension_type(settings)).is_some()
}

pub(in crate::game) fn installed_dimensions() -> Vec<ModDimension> {
    mod_api::__private::dimension_declarations()
        .into_iter()
        .map(|(id, settings)| {
            let stem = json!({
                "type": id,
                "generator": {
                    "type": settings.generator,
                    "settings": settings.noise_settings,
                    "biome_source": {"type": "minecraft:fixed", "biome": settings.biome},
                },
            });
            ModDimension {
                id,
                stem: stem.to_string(),
                dimension_type: dimension_type(&settings).to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    use super::*;
    #[cfg(test)]
    use crate::game::api_values::install_parts;
    #[cfg(test)]
    use mod_api::*;

    /// A name that is no safe folder of the world refuses the mods at startup, before any world
    /// opens: a level's folder is its id under the world's `dimensions` folder.
    #[test]
    fn a_name_that_escapes_the_worlds_folder_refuses_registration() {
        use mod_api::DimensionSettings;
        const BAD: [&str; 10] = [
            "../minecraft/overworld",
            "a/../../b",
            "a/../b",
            "/abs",
            "trailing/",
            "a//b",
            ".",
            "..",
            "Pocket",
            "my pocket",
        ];
        struct Invalid;
        #[mod_api::__subscriptions]
        impl Hooks for Invalid {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                for name in BAD {
                    r.register_dimension(name, DimensionSettings::default());
                }
            }
        }
        let errors = install_parts(&[ModEntry::new("test", Box::new(Invalid))]).unwrap_err();
        assert_eq!(errors.len(), BAD.len(), "{errors:?}");
        for (error, name) in errors.iter().zip(BAD) {
            assert!(error.contains(&format!("test:{name}\"")), "{name}: {error}");
        }
    }

    /// Nested names with the characters ids allow are fine.
    #[test]
    fn a_nested_name_registers() {
        use mod_api::DimensionSettings;
        struct Nested;
        #[mod_api::__subscriptions]
        impl Hooks for Nested {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_dimension("a/b", DimensionSettings::default());
                r.register_dimension("test:deep/pocket-1.x", DimensionSettings::default());
            }
        }
        install_parts(&[ModEntry::new("test", Box::new(Nested))]).unwrap();
        let ids: Vec<String> = installed_dimensions().into_iter().map(|d| d.id).collect();
        assert_eq!(ids, ["test:a/b", "test:deep/pocket-1.x"]);
    }

    /// Each dimension a mod may not register: every one is refused at
    /// startup, naming the mod.
    #[test]
    fn invalid_dimensions_refuse_registration() {
        use mod_api::DimensionSettings;
        struct Invalid;
        #[mod_api::__subscriptions]
        impl Hooks for Invalid {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_dimension("pocket", DimensionSettings::default());
                r.register_dimension("pocket", DimensionSettings::default());
                r.register_dimension("other:pocket", DimensionSettings::default());
                r.register_dimension("", DimensionSettings::default());
                r.register_dimension(
                    "uneven",
                    DimensionSettings {
                        height: 40,
                        logical_height: 40,
                        ..Default::default()
                    },
                );
                r.register_dimension(
                    "low_ceiling",
                    DimensionSettings {
                        height: 64,
                        ..Default::default()
                    },
                );
                r.register_dimension(
                    "too_tall",
                    DimensionSettings {
                        min_y: 2016,
                        height: i32::MAX - 15,
                        logical_height: 0,
                        ..Default::default()
                    },
                );
                r.register_dimension(
                    "custom",
                    DimensionSettings {
                        generator: "test:void".into(),
                        ..Default::default()
                    },
                );
            }
        }
        let errors = install_parts(&[ModEntry::new("test", Box::new(Invalid))]).unwrap_err();
        assert_eq!(errors.len(), 7, "{errors:?}");
        assert!(errors[0].contains("test registered dimension test:pocket twice"));
        assert!(errors[1].contains("namespace"));
        assert!(errors[2].contains("no name"));
        assert!(errors[3].contains("test:uneven"));
        assert!(errors[4].contains("test:low_ceiling"));
        assert!(errors[5].contains("test:too_tall"));
        assert!(errors[6].contains("test:custom with the unknown generator \"test:void\""));
    }

    #[cfg(test)]
    struct Void;
    #[cfg(test)]
    impl mod_api::ChunkGenerator for Void {
        fn build_terrain(&self, _: &mut dyn mod_api::ChunkAccess) {}
    }

    /// A generator's name is held to the same rule, so `Void` or `../x` refuses the mods too.
    #[test]
    fn a_generator_name_that_is_no_folder_refuses_registration() {
        const BAD: [&str; 6] = ["../x", "a//b", "/abs", "Void", "my void", "."];
        struct Invalid;
        #[mod_api::__subscriptions]
        impl Hooks for Invalid {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                for name in BAD {
                    r.register_chunk_generator(name, Void);
                }
                r.register_chunk_generator("gen/void_2", Void);
            }
        }
        let errors = install_parts(&[ModEntry::new("test", Box::new(Invalid))]).unwrap_err();
        assert_eq!(errors.len(), BAD.len(), "{errors:?}");
        for (error, name) in errors.iter().zip(BAD) {
            assert!(error.contains(&format!("test:{name}\"")), "{name}: {error}");
        }
    }

    /// Each generator a mod may not register: every one is refused at startup,
    /// naming the mod.
    #[test]
    fn invalid_chunk_generators_refuse_registration() {
        struct Invalid;
        #[mod_api::__subscriptions]
        impl Hooks for Invalid {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_chunk_generator("void", Void);
                r.register_chunk_generator("void", Void);
                r.register_chunk_generator("other:void", Void);
                r.register_chunk_generator("", Void);
            }
        }
        let errors = install_parts(&[ModEntry::new("test", Box::new(Invalid))]).unwrap_err();
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("test registered chunk generator test:void twice"));
        assert!(errors[1].contains("test registered chunk generator \"other:void\""));
        assert!(errors[2].contains("test registered a chunk generator with no name"));
    }

    /// A mod with vanilla's id can't take over every noise level.
    #[test]
    fn the_noise_generator_refuses_registration() {
        struct Noise;
        #[mod_api::__subscriptions]
        impl Hooks for Noise {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_chunk_generator("noise", Void);
            }
        }
        let errors = install_parts(&[ModEntry::new("minecraft", Box::new(Noise))]).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("minecraft:noise, which is the game's own"));
    }

    /// The generator check runs after every mod registered.
    #[test]
    fn a_later_mods_generator_satisfies_an_earlier_mods_dimension() {
        struct Dimension;
        #[mod_api::__subscriptions]
        impl Hooks for Dimension {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_dimension(
                    "pocket",
                    mod_api::DimensionSettings {
                        generator: "later:void".into(),
                        ..Default::default()
                    },
                );
            }
        }
        struct Generator;
        #[mod_api::__subscriptions]
        impl Hooks for Generator {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_chunk_generator("void", Void);
            }
        }
        install_parts(&[
            ModEntry::new("earlier", Box::new(Dimension)),
            ModEntry::new("later", Box::new(Generator)),
        ])
        .unwrap();
        let dimensions = installed_dimensions();
        let stem: serde_json::Value = serde_json::from_str(&dimensions[0].stem).unwrap();
        assert_eq!(stem["generator"]["type"], "later:void");
        let generators = crate::game::native_chunk_generator::installed_chunk_generators();
        assert!(generators.contains_key("later:void"));
    }

    /// What the engine calls is the mod's own code, under its namespaced id.
    #[test]
    fn a_registered_generator_reaches_the_engine() {
        use mod_api::{ChunkAccess, ChunkPos};
        // The engine's chunk: what the worker threads hand the generator.
        use horizon_world::registry::BlockStateId;
        use horizon_world::worldgen::mod_generators::ChunkAccess as EngineChunk;
        struct Marker(mod_api::BlockKey);
        impl mod_api::ChunkGenerator for Marker {
            fn build_terrain(&self, chunk: &mut dyn ChunkAccess) {
                let state = chunk.state(self.0);
                assert_eq!(chunk.metadata().name(state.block()), Ok("minecraft:stone"));
                assert!(chunk.set_block([1, chunk.min_y(), 2], state));
                let invalid =
                    mod_api::BlockState::from(mod_api::BlockStateRecord::new(self.0, u16::MAX));
                assert!(!chunk.set_block([2, chunk.min_y(), 2], invalid));
            }
        }
        struct Writes;
        #[mod_api::__subscriptions]
        impl Hooks for Writes {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                let block = r.block_key("minecraft:stone");
                r.register_chunk_generator("test:marker", Marker(block));
            }
        }
        struct Written(
            Vec<([i32; 3], BlockStateId)>,
            &'static horizon_world::registry::BlockRegistry,
        );
        impl EngineChunk for Written {
            fn registry(&self) -> &'static horizon_world::registry::BlockRegistry {
                self.1
            }
            fn pos(&self) -> ChunkPos {
                ChunkPos::new(0, 0)
            }
            fn seed(&self) -> i64 {
                0
            }
            fn min_y(&self) -> i32 {
                -64
            }
            fn height(&self) -> i32 {
                384
            }
            fn state(&self, name: &str) -> BlockStateId {
                horizon_world::worldgen::structure::legacy::block_state(self.1, name, &[])
            }
            fn state_with(&self, name: &str, properties: &[(&str, &str)]) -> BlockStateId {
                horizon_world::worldgen::structure::legacy::block_state(self.1, name, properties)
            }
            fn set_block(&mut self, pos: [i32; 3], state: BlockStateId) -> bool {
                self.0.push((pos, state));
                true
            }
        }
        install_parts(&[ModEntry::new("test", Box::new(Writes))]).unwrap();
        let generators = crate::game::native_chunk_generator::installed_chunk_generators();
        let facts = r#"{"defaults":{"minecraft:air":0,"minecraft:stone":1},"states":[["a",true,false,"","minecraft:air"],["s",false,false,"","minecraft:stone"]]}"#;
        let registry = Box::leak(Box::new(
            horizon_world::registry::BlockRegistry::from_facts_json(facts.as_bytes()).unwrap(),
        ));
        crate::game::api_values::resolve_block_definitions(registry).unwrap();
        let generator = generators["test:marker"].clone();
        let writes = std::thread::spawn(move || {
            let mut chunk = Written(Vec::new(), registry);
            generator.build_terrain(&mut chunk);
            chunk.0
        })
        .join()
        .unwrap();
        assert_eq!(writes, [([1, -64, 2], BlockStateId(1))]);
    }

    /// The vanilla levels are in the `minecraft` namespace, so only a mod with
    /// that id gets as far as naming one.
    #[test]
    fn a_vanilla_level_refuses_registration() {
        struct Vanilla;
        #[mod_api::__subscriptions]
        impl Hooks for Vanilla {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_dimension("the_end", mod_api::DimensionSettings::default());
            }
        }
        let errors = install_parts(&[ModEntry::new("minecraft", Box::new(Vanilla))]).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("minecraft:the_end, which is a vanilla level"));
    }

    /// A registered dimension reaches the engine as the level stem and
    /// dimension type a data pack would give.
    #[test]
    fn a_registered_dimension_reaches_the_engine_as_pack_entries() {
        use horizon_world::worldgen::overrides::{DIMENSION_TYPES, DIMENSIONS, WorldgenOverrides};
        struct Pocket;
        #[mod_api::__subscriptions]
        impl Hooks for Pocket {
            fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
                r.register_dimension(
                    "test:pocket",
                    mod_api::DimensionSettings {
                        min_y: -128,
                        height: 448,
                        logical_height: 320,
                        has_skylight: false,
                        has_ceiling: true,
                        noise_settings: "minecraft:caves".into(),
                        biome: "minecraft:lush_caves".into(),
                        ..Default::default()
                    },
                );
            }
        }
        install_parts(&[ModEntry::new("test", Box::new(Pocket))]).unwrap();
        let dimensions = installed_dimensions();
        assert_eq!(dimensions.len(), 1);
        let mut overrides = WorldgenOverrides::default();
        overrides.add_mod_dimension(&dimensions[0]);
        let height = overrides.level_height("test:pocket").unwrap();
        assert_eq!(
            (
                height.min_y,
                height.height,
                height.logical_height,
                height.has_skylight,
                height.has_ceiling
            ),
            (-128, 448, 320, false, true)
        );
        assert_eq!(overrides.entries(DIMENSION_TYPES).count(), 1);
        let (id, stem) = overrides.entries(DIMENSIONS).next().unwrap();
        assert_eq!(id, "test:pocket");
        let stem: serde_json::Value = serde_json::from_str(stem).unwrap();
        assert_eq!(
            stem,
            serde_json::json!({
                "type": "test:pocket",
                "generator": {
                    "type": "minecraft:noise",
                    "settings": "minecraft:caves",
                    "biome_source": {"type": "minecraft:fixed", "biome": "minecraft:lush_caves"},
                },
            })
        );
    }
}
