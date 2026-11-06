//! Structure types a mod registers (vanilla's `StructureType` with its own
//! `Structure.findGenerationPoint` and `StructurePiece`s), handed over once at
//! startup and looked up by `StructureRegistry::new` before `Unsupported`. The
//! game keeps the one piece mechanism it has (`Piece::Legacy`): a mod type's
//! pieces are legacy pieces behind an adapter, so the start cache, the saving,
//! the cross-chunk placement and the beardifier treat them like a fortress
//! piece's. Runs on the worldgen worker threads, so a type keeps no mutable
//! state and its contexts carry no `Host`.
use super::legacy::LegacyPiece;
use super::start::StartContext;
use horizon_core::nbt::Compound;
use horizon_core::{
    position::ChunkPos,
    random::{LegacyRandom, WorldgenRandom},
};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

/// A structure type's engine side: vanilla's `StructureType` bound to the
/// pieces' `StructurePieceType` loader, as the executable's adapter sees it.
/// `Send + Sync` because neighbouring chunks build at once on worker threads;
/// the same seed gives the same pieces.
pub trait ModStructureType: Send + Sync {
    /// `Structure.findGenerationPoint` with `generatePieces`: the start
    /// position and at least one piece for `chunk`, or none for "no start
    /// here". The placement checks stay the game's own: the set's spacing, the
    /// structure's biomes and its `terrain_adaptation`.
    fn find_generation_point(
        &self,
        context: &mut dyn StartContext,
        chunk: ChunkPos,
        // The structure's own JSON config, as the data pack has it.
        config: &Arc<serde_json::Value>,
        // Seeded as vanilla seeds it for the chunk (`setLargeFeatureSeed`).
        random: &mut WorldgenRandom<LegacyRandom>,
    ) -> Option<([i32; 3], Vec<Box<dyn LegacyPiece>>)>;
    /// The `StructurePieceType` loader (`StructurePieceType.load`): a saved
    /// piece back exactly as it was saved. There is no rebuild-from-seed
    /// fallback: every piece type has its loader.
    fn load_piece(&self, tag: &Compound) -> Option<Box<dyn LegacyPiece>>;
}

/// The installed types, `modid:tower` → type. Set once, before any world opens
/// (the executable hands the mods' registrations over), and read by
/// `StructureRegistry::new` while decoding a bundle's structures.
pub type ModStructureTypes = BTreeMap<String, Arc<dyn ModStructureType>>;

static MOD_TYPES: OnceLock<ModStructureTypes> = OnceLock::new();

#[cfg(test)]
thread_local! {
    /// The types a test installs, in place of the process-wide table.
    static TEST_TYPES: std::cell::Cell<Option<&'static ModStructureTypes>> =
        const { std::cell::Cell::new(None) };
}

/// Install the process-wide table once; a second installation keeps the first.
pub fn install_mod_structure_types(types: ModStructureTypes) -> Result<(), ModStructureTypes> {
    MOD_TYPES.set(types)
}

/// The installed structure types: empty until the executable installs the
/// mods', and a test's own while one is installed.
pub fn mod_structure_types() -> &'static ModStructureTypes {
    #[cfg(test)]
    if let Some(types) = TEST_TYPES.get() {
        return types;
    }
    MOD_TYPES.get().unwrap_or(&EMPTY)
}

static EMPTY: ModStructureTypes = BTreeMap::new();

#[cfg(test)]
/// Runs `with` over a test's own types, as the registry reads them.
pub fn with_test_types<T>(types: &ModStructureTypes, with: impl FnOnce() -> T) -> T {
    let leaked: &'static ModStructureTypes = Box::leak(Box::new(types.clone()));
    let prev = TEST_TYPES.replace(Some(leaked));
    let out = with();
    TEST_TYPES.set(prev);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::structure::legacy::{PieceContext, Result};
    use crate::worldgen::structure::start::StructureKind;
    use horizon_core::bounding_box::BoundingBox;
    use serde_json::json;
    use std::collections::BTreeSet;

    /// A type whose every piece is its box: positions it got, in a tag.
    struct Boxed(Vec<[i32; 3]>);

    impl ModStructureType for Boxed {
        fn find_generation_point(
            &self,
            _context: &mut dyn StartContext,
            chunk: ChunkPos,
            _config: &Arc<serde_json::Value>,
            _random: &mut WorldgenRandom<LegacyRandom>,
        ) -> Option<([i32; 3], Vec<Box<dyn LegacyPiece>>)> {
            let origin = [chunk.x * 16, 64, chunk.z * 16];
            let pieces = self
                .0
                .iter()
                .map(|&offset| {
                    Box::new(SavedPiece {
                        id: "testmod:boxed".to_owned(),
                        min: offset,
                        max: [offset[0] + 1, offset[1] + 1, offset[2] + 1],
                    }) as Box<dyn LegacyPiece>
                })
                .collect();
            Some((origin, pieces))
        }
        fn load_piece(&self, tag: &Compound) -> Option<Box<dyn LegacyPiece>> {
            let int_array = |key: &str| -> Option<[i32; 3]> {
                let value = tag.get(&key.into())?;
                let horizon_core::nbt::Tag::IntArray(a) = value else {
                    return None;
                };
                Some([a[0], a[1], a[2]])
            };
            Some(Box::new(SavedPiece {
                id: "testmod:boxed".to_owned(),
                min: int_array("Min")?,
                max: int_array("Max")?,
            }))
        }
    }

    /// The adapter's saved piece shape: a box that saves and loads back.
    struct SavedPiece {
        id: String,
        min: [i32; 3],
        max: [i32; 3],
    }
    impl LegacyPiece for SavedPiece {
        fn bounding_box(&self) -> BoundingBox {
            BoundingBox::new(self.min, self.max)
        }
        fn type_name(&self) -> &'static str {
            Box::leak(self.id.clone().into_boxed_str())
        }
        fn save(&self) -> Option<Compound> {
            use horizon_core::nbt::Tag;
            let mut tag = super::super::legacy::piece_tag(&self.id, self.bounding_box(), None, 0);
            tag.insert("Min".into(), Tag::IntArray(self.min.to_vec()));
            tag.insert("Max".into(), Tag::IntArray(self.max.to_vec()));
            Some(tag)
        }
        fn post_process(&self, _: &mut PieceContext<'_>) -> Result<()> {
            Ok(())
        }
    }
    impl std::fmt::Debug for SavedPiece {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SavedPiece").field("id", &self.id).finish()
        }
    }

    /// A bundle structure whose `"type"` a registered mod type answers, keeps
    /// the structure's JSON config and rebuilds its pieces by it.
    #[test]
    fn registry_resolves_a_mod_type_and_keeps_its_config() {
        use super::super::start::StructureRegistry;
        let types: ModStructureTypes = BTreeMap::from([(
            "testmod:boxed".to_owned(),
            Arc::new(Boxed(vec![[0, 0, 0], [8, 0, 8]])) as Arc<dyn ModStructureType>,
        )]);
        let structures = BTreeMap::from([(
            "testmod:tower".to_owned(),
            json!({"type": "testmod:boxed", "biomes": "#minecraft:is_forest"}).to_string(),
        )]);
        let registry = with_test_types(&types, || {
            StructureRegistry::new(
                &structures,
                &BTreeMap::new(),
                BTreeMap::new(),
                &|_| Ok(BTreeSet::new()),
                &types,
            )
        })
        .unwrap();
        assert_eq!(registry.structures.len(), 1);
        let kind = &registry.structures[0].kind;
        assert!(matches!(
            kind,
            StructureKind::Mod(m) if m.config["type"] == "testmod:boxed"
        ));
    }

    /// An unknown `"type"` stays `Unsupported`, placed by no one.
    #[test]
    fn unknown_type_stays_unsupported() {
        use super::super::start::StructureRegistry;
        let structures = BTreeMap::from([(
            "testmod:tower".to_owned(),
            json!({"type": "testmod:gone", "biomes": "#minecraft:is_forest"}).to_string(),
        )]);
        let registry = StructureRegistry::new(
            &structures,
            &BTreeMap::new(),
            BTreeMap::new(),
            &|_| Ok(BTreeSet::new()),
            &ModStructureTypes::new(),
        )
        .unwrap();
        assert!(matches!(
            registry.structures[0].kind,
            StructureKind::Unsupported(ref id) if id == "testmod:gone"
        ));
    }

    /// A piece a type's loader rebuilds comes back with its box and its
    /// type id, as it was saved.
    #[test]
    fn load_piece_rebuilds_the_saved_piece() {
        use horizon_core::nbt::Tag;
        let types: ModStructureTypes = BTreeMap::from([(
            "testmod:boxed".to_owned(),
            Arc::new(Boxed(vec![])) as Arc<dyn ModStructureType>,
        )]);
        let ty = &types["testmod:boxed"];
        let mut tag = super::super::legacy::piece_tag(
            "testmod:boxed",
            BoundingBox::new([1, 64, 2], [3, 66, 4]),
            None,
            0,
        );
        tag.insert("Min".into(), Tag::IntArray(vec![1, 64, 2]));
        tag.insert("Max".into(), Tag::IntArray(vec![3, 66, 4]));
        let piece = ty.load_piece(&tag).unwrap();
        assert_eq!(
            piece.bounding_box(),
            BoundingBox::new([1, 64, 2], [3, 66, 4])
        );
        assert_eq!(piece.type_name(), "testmod:boxed");
    }
}
