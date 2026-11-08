use horizon_world::worldgen::biome_source::{
    BiomeSet, BiomeSourceCatalog, BiomeSourceDefinition, CheckerboardBiomeSource, CheckerboardError,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn direct_biome_lists_preserve_large_ordered_duplicates() {
    let catalog = BiomeSourceCatalog::new(
        &BTreeSet::from(["minecraft:plains".into(), "minecraft:desert".into()]),
        &BTreeMap::new(),
        &BTreeSet::new(),
    )
    .unwrap();
    for len in [8192, 8193, 9000] {
        let mut names = vec!["minecraft:plains".to_owned(); len];
        names[1] = "minecraft:desert".into();
        names[len - 1] = "minecraft:desert".into();
        let json = serde_json::json!({"type":"checkerboard", "scale":0,"biomes":names}).to_string();
        let definition = catalog.decode(&json).unwrap();
        let BiomeSourceDefinition::Checkerboard {
            biomes: BiomeSet::Direct(decoded),
            scale,
        } = &definition
        else {
            panic!("wrong source")
        };
        assert_eq!(decoded, &names);
        assert_eq!(
            catalog
                .decode(&catalog.encode(&definition).unwrap())
                .unwrap(),
            definition
        );
        let source = CheckerboardBiomeSource::new(decoded.clone(), *scale).unwrap();
        // scale 0 shifts by 2: cells 0, 1, 2, -1, -2 and len + 1.
        for (quart, expected) in [
            ([0, 0, 0], "minecraft:plains"),
            ([4, 0, 0], "minecraft:desert"),
            ([8, 0, 0], "minecraft:plains"),
            ([-1, 99, 0], "minecraft:desert"),
            ([-5, 0, 0], "minecraft:plains"),
            ([(len as i32 + 1) * 4, 0, 0], "minecraft:desert"),
        ] {
            assert_eq!(source.noise_biome(quart).unwrap(), expected);
        }
        let unknown = json.replace("minecraft:desert", "minecraft:unknown");
        assert!(catalog.decode(&unknown).is_err());
    }
}

#[test]
fn direct_biome_empty_sampling_is_refused() {
    let source = CheckerboardBiomeSource::<()>::new(vec![], 0).unwrap();
    assert_eq!(
        source.noise_biome([0; 3]),
        Err(CheckerboardError::EmptyBiomes)
    );
}

#[test]
fn direct_biome_lengths_do_not_truncate_to_i32() {
    // Zero-sized entries exercise huge lengths without allocating huge memory.
    for len in [i32::MAX as usize + 1, u32::MAX as usize + 1] {
        let source = CheckerboardBiomeSource::new(vec![(); len], 0).unwrap();
        assert_eq!(source.noise_biome([-1, 0, 0]), Ok(&()));
        assert_eq!(source.noise_biome([0; 3]), Ok(&()));
    }
}
