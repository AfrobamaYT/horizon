use super::*;
use std::fs;

fn point(biome: &str, low: f32) -> mod_api::BiomePoint {
    mod_api::BiomePoint {
        biome: biome.into(),
        temperature: (low, 0.1),
        humidity: (-0.3, 0.7),
        continentalness: (-1.9, 1.3),
        erosion: (0.15, 0.25),
        depth: (0.0, 1.0),
        weirdness: (-2.0, 2.0),
        offset: 0.375,
    }
}

fn declaration(mod_id: &str, id: &str, dimension: &str, weight: u32) -> BiomeRegionDeclaration {
    BiomeRegionDeclaration {
        mod_id: mod_id.into(),
        id: id.into(),
        region: mod_api::BiomeRegion {
            dimension: dimension.into(),
            weight,
            points: vec![point(&format!("{mod_id}:hills"), -0.45)],
        },
    }
}

/// Two mods' Overworld regions, sorted as the declarations install them.
fn overworld() -> Vec<BiomeRegionDeclaration> {
    vec![
        declaration("alpha", "alpha:marsh", "minecraft:overworld", 3),
        declaration("zeta", "zeta:canyon", "minecraft:overworld", 1),
    ]
}

fn mods<'a>(ids: &[&'a str]) -> BTreeSet<&'a str> {
    ids.iter().copied().collect()
}

/// The Overworld's region list in an encoded record.
fn regions(tag: &mut Tag) -> &mut Vec<Tag> {
    match tag
        .get_mut("dimensions")
        .and_then(|d| d.get_mut("minecraft:overworld"))
        .and_then(|l| l.get_mut("regions"))
    {
        Some(Tag::List(regions)) => regions,
        _ => panic!("the layout has regions"),
    }
}

fn put(tag: &mut Tag, key: &str, value: Tag) {
    if let Tag::Compound(map) = tag {
        map.insert(key.into(), value);
    }
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_new_worlds_record_holds_only_populated_dimensions_in_declared_order() {
    let mut declared = overworld();
    declared.push(declaration("beta", "beta:ash", "minecraft:nether", 7));
    declared.sort_by(|a, b| (&a.mod_id, &a.id).cmp(&(&b.mod_id, &b.id)));
    let record = BiomeRegions::from_declarations(&declared);
    let overworld = record.layout("minecraft:overworld").unwrap();
    assert_eq!(overworld.selector, SelectorParams::current());
    let regions: Vec<_> = overworld
        .regions
        .iter()
        .map(|r| (r.owner.as_str(), r.id.as_str(), r.weight, r.substituted))
        .collect();
    assert_eq!(
        regions,
        [
            ("alpha", "alpha:marsh", 3, false),
            ("zeta", "zeta:canyon", 1, false)
        ]
    );
    assert_eq!(overworld.regions[0].points[0].intervals[0], (-0.45, 0.1));
    let nether = record.layout("minecraft:nether").unwrap();
    assert_eq!(nether.regions.len(), 1);
    assert_eq!(nether.regions[0].id, "beta:ash");
    assert!(record.layout("minecraft:the_end").is_none());
    // No declarations, no layout in any dimension.
    assert_eq!(
        BiomeRegions::from_declarations(&[]),
        BiomeRegions::default()
    );
}

#[test]
fn the_record_reads_back_every_field_bit_exact() {
    let dir = tempfile::tempdir().unwrap();
    let mut record = BiomeRegions::from_declarations(&overworld());
    let layout = Arc::make_mut(record.dimensions.get_mut("minecraft:overworld").unwrap());
    layout.selector = SelectorParams {
        version: REGION_SELECTOR_VERSION,
        cell: 64,
        salt: "test:salt".into(),
        vanilla_weight: 12,
    };
    // Values with no short decimal form: a lossy float path would not return these bits.
    layout.regions[0].points[0].intervals[3] = (1.0e-7, f32::from_bits(0x3e4c_cccd));
    layout.regions[0].points[0].offset = f32::from_bits(0x3eaa_aaab);
    layout.regions[1].substituted = true;
    record.write(dir.path()).unwrap();
    assert_eq!(BiomeRegions::read(dir.path()).unwrap(), Some(record));
}

#[test]
fn an_absent_record_is_no_regions() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(BiomeRegions::read(dir.path()), Ok(None));
    assert_eq!(open_with(dir.path(), &mods(&[]), &overworld()), Ok(None));
}

#[test]
fn a_malformed_record_is_refused_and_left_untouched() {
    let good = BiomeRegions::from_declarations(&overworld()).encode();
    let edit = |change: &dyn Fn(&mut Tag)| {
        let mut tag = good.clone();
        change(&mut tag);
        tag
    };
    let variants: Vec<(&str, Option<Tag>)> = vec![
        ("not nbt", None),
        (
            "unknown version",
            Some(edit(&|t: &mut Tag| put(t, "version", Tag::Int(2)))),
        ),
        (
            "ill-typed dimensions",
            Some(edit(&|t: &mut Tag| put(t, "dimensions", Tag::Int(0)))),
        ),
        (
            "missing weight",
            Some(edit(&|t: &mut Tag| {
                if let Tag::Compound(map) = &mut regions(t)[0] {
                    map.remove(&JavaString::from("weight"));
                }
            })),
        ),
        (
            "weight 0",
            Some(edit(&|t: &mut Tag| {
                put(&mut regions(t)[0], "weight", Tag::Int(0))
            })),
        ),
        (
            "point outside -2 to 2",
            Some(edit(&|t: &mut Tag| {
                if let Some(Tag::List(points)) = regions(t)[0].get_mut("points") {
                    put(
                        &mut points[0],
                        "humidity",
                        Tag::List(vec![Tag::Float(0f32.to_bits()), Tag::Float(3f32.to_bits())]),
                    );
                }
            })),
        ),
        ("unsorted", Some(edit(&|t: &mut Tag| regions(t).swap(0, 1)))),
        (
            "duplicate",
            Some(edit(&|t: &mut Tag| {
                let list = regions(t);
                list[1] = list[0].clone();
            })),
        ),
        (
            "substituted for another mod",
            Some(edit(&|t: &mut Tag| {
                put(
                    &mut regions(t)[0],
                    "substituted",
                    Tag::compound([("with", string(VANILLA)), ("missing_mod", string("zeta"))]),
                )
            })),
        ),
    ];
    for (name, tag) in variants {
        let dir = tempfile::tempdir().unwrap();
        let file = path(dir.path());
        match tag {
            Some(tag) => write_dat_durable(&file, tag).unwrap(),
            None => {
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(&file, b"not nbt").unwrap();
            }
        }
        let bytes = fs::read(&file).unwrap();
        let everyone = mods(&["alpha", "zeta"]);
        assert!(BiomeRegions::read(dir.path()).is_err(), "{name}");
        assert!(
            open_with(dir.path(), &everyone, &overworld()).is_err(),
            "{name}"
        );
        assert!(
            choose_with(dir.path(), &mods(&[]), &[], true, true).is_err(),
            "{name}"
        );
        assert!(unchosen_missing(dir.path()).is_empty(), "{name}");
        assert_eq!(fs::read(&file).unwrap(), bytes, "{name}");
        assert_eq!(
            entries(file.parent().unwrap()),
            ["biome_regions.dat"],
            "{name}"
        );
    }
}

#[test]
fn a_missing_mods_region_refuses_the_open_naming_mod_and_region() {
    let dir = tempfile::tempdir().unwrap();
    BiomeRegions::from_declarations(&overworld())
        .write(dir.path())
        .unwrap();
    let only_alpha = vec![overworld().remove(0)];
    let why = open_with(dir.path(), &mods(&["alpha"]), &only_alpha).unwrap_err();
    assert!(
        why.contains("zeta:canyon") && why.contains("mod zeta"),
        "{why}"
    );
    assert!(!why.contains("alpha"), "{why}");
    let record = BiomeRegions::read(dir.path()).unwrap().unwrap();
    assert_eq!(
        record.differences(&mods(&["alpha"]), &only_alpha),
        Differences {
            missing: vec![("zeta".into(), "zeta:canyon".into())],
            ..Differences::default()
        }
    );
}

#[test]
fn choosing_vanilla_is_saved_and_opens_from_then_on() {
    let dir = tempfile::tempdir().unwrap();
    BiomeRegions::from_declarations(&overworld())
        .write(dir.path())
        .unwrap();
    let only_alpha = vec![overworld().remove(0)];
    let alpha = mods(&["alpha"]);
    // Refusing ("missing = refuse") records nothing.
    choose_with(dir.path(), &alpha, &only_alpha, false, false).unwrap();
    assert!(open_with(dir.path(), &alpha, &only_alpha).is_err());
    choose_with(dir.path(), &alpha, &only_alpha, true, false).unwrap();
    let opened = open_with(dir.path(), &alpha, &only_alpha).unwrap().unwrap();
    let layout = opened.layout("minecraft:overworld").unwrap();
    // The region keeps its slot and weight; only the substitution is new.
    assert_eq!(
        layout
            .regions
            .iter()
            .map(|r| (r.id.as_str(), r.weight, r.substituted))
            .collect::<Vec<_>>(),
        [("alpha:marsh", 3, false), ("zeta:canyon", 1, true)]
    );
    // The mod's return is reported, and the substitution stays.
    let everyone = mods(&["alpha", "zeta"]);
    let differences = opened.differences(&everyone, &overworld());
    assert_eq!(differences.returned, ["zeta:canyon"]);
    assert!(differences.missing.is_empty());
    let reopened = open_with(dir.path(), &everyone, &overworld()).unwrap();
    assert_eq!(reopened, Some(opened));
}

#[test]
fn changed_added_and_removed_regions_are_reported_and_the_record_wins() {
    let dir = tempfile::tempdir().unwrap();
    BiomeRegions::from_declarations(&overworld())
        .write(dir.path())
        .unwrap();
    let saved = BiomeRegions::read(dir.path()).unwrap().unwrap();
    let mut now = overworld();
    now[0].region.points[0].offset = 0.5; // alpha's points changed
    now.remove(1); // zeta no longer declares its region
    now.push(declaration("zeta", "zeta:dunes", "minecraft:nether", 2)); // a new region
    let everyone = mods(&["alpha", "zeta"]);
    let mut changed = saved.differences(&everyone, &now).changed;
    changed.sort();
    assert_eq!(changed, ["alpha:marsh", "zeta:canyon", "zeta:dunes"]);
    assert_eq!(
        open_with(dir.path(), &everyone, &now).unwrap(),
        Some(saved.clone())
    );
    // A changed weight is a difference too.
    let mut heavier = overworld();
    heavier[1].region.weight = 9;
    assert_eq!(
        saved.differences(&everyone, &heavier).changed,
        ["zeta:canyon"]
    );
    assert_eq!(
        saved.differences(&everyone, &overworld()),
        Differences::default()
    );
}

#[test]
fn adopting_takes_installed_regions_and_keeps_saved_weights() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = BiomeRegions::from_declarations(&overworld());
    let layout = Arc::make_mut(saved.dimensions.get_mut("minecraft:overworld").unwrap());
    layout.regions[0].weight = 40; // the owner's own weight for alpha
    layout.regions[1].substituted = true;
    layout.selector.cell = 64;
    saved.write(dir.path()).unwrap();
    let mut now = overworld();
    now[0].region.points[0].offset = 0.5;
    now.push(declaration("zeta", "zeta:dunes", "minecraft:nether", 2));
    let everyone = mods(&["alpha", "zeta"]);
    choose_with(dir.path(), &everyone, &now, false, true).unwrap();
    let adopted = BiomeRegions::read(dir.path()).unwrap().unwrap();
    let overworld = adopted.layout("minecraft:overworld").unwrap();
    assert_eq!(overworld.selector.cell, 64);
    assert_eq!(
        overworld
            .regions
            .iter()
            .map(|r| (r.id.as_str(), r.weight, r.substituted))
            .collect::<Vec<_>>(),
        [("alpha:marsh", 40, false), ("zeta:canyon", 1, false)]
    );
    assert_eq!(overworld.regions[0].points[0].offset, 0.5);
    let nether = adopted.layout("minecraft:nether").unwrap();
    assert_eq!(
        (nether.regions[0].id.as_str(), nether.regions[0].weight),
        ("zeta:dunes", 2)
    );
    assert_eq!(adopted.differences(&everyone, &now), Differences::default());
}

#[test]
fn interrupted_replacement_refuses_to_drop_the_layout() {
    let dir =
        std::env::temp_dir().join(format!("horizon-region-interrupted-{}", std::process::id()));
    fs::create_dir_all(path(&dir).parent().unwrap()).unwrap();
    for name in ["biome_regions.dat_old", "biome_regions.dat_new"] {
        let leftover = path(&dir).with_file_name(name);
        fs::write(&leftover, b"saved layout").unwrap();
        let why = BiomeRegions::read(&dir).unwrap_err();
        assert!(why.contains(name), "{why}");
        assert_eq!(fs::read(&leftover).unwrap(), b"saved layout");
        assert!(!path(&dir).exists());
        fs::remove_file(leftover).unwrap();
    }
}

#[test]
fn choices_refuse_a_world_already_open_in_another_session() {
    let scratch = tempfile::tempdir().unwrap();
    let dir = scratch.path();
    let lock = super::super::SessionLock::acquire(&dir).unwrap();
    assert!(choose_with(&dir, &mods(&[]), &[], true, true).is_err());
    assert!(!path(&dir).exists());
    drop(lock);
    assert!(choose_with(&dir, &mods(&[]), &[], true, true).is_ok());
}
