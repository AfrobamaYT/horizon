use super::super::missing_biome_regions::tests::record;
use super::super::tests::Fixture;
use super::*;

fn adopt_offered(f: &Fixture) -> bool {
    f.title
        .buttons(480.0, 262.0)
        .iter()
        .any(|b| b.label == lang::get("horizon.biome_regions.adopt.button"))
}

#[test]
fn edit_world_offers_the_installed_regions_only_when_they_differ() {
    let mut f = Fixture::new();
    f.world("Plain", "Plain");
    f.world("Moor", "Moor");
    // A world without a record, and no region mod installed: nothing to adopt.
    let plain = list_worlds(&f.title.saves)
        .into_iter()
        .find(|w| w.folder == "Plain")
        .unwrap();
    f.title.edit_world("Plain".into(), plain);
    assert!(!adopt_offered(&f));
    // Its saved region's mod is not installed, so adopting would drop the region.
    let file = record(&f, "Moor", false);
    let moor = list_worlds(&f.title.saves)
        .into_iter()
        .find(|w| w.folder == "Moor")
        .unwrap();
    f.title.edit_world("Renamed".into(), moor.clone());
    assert!(adopt_offered(&f));
    // Cancel goes back to Edit World with the typed name; the record is as it was.
    let before = std::fs::read(&file).unwrap();
    f.click(&lang::get("horizon.biome_regions.adopt.button"));
    assert!(matches!(f.title.mode, Mode::AdoptBiomeRegions { .. }));
    f.click(&lang::get_or("gui.cancel", "Cancel"));
    assert!(matches!(&f.title.mode, Mode::Edit { name, .. } if name == "Renamed"));
    assert_eq!(std::fs::read(&file).unwrap(), before);
    // Adopting writes the choice into the world, and the opt-in is no longer offered.
    f.click(&lang::get("horizon.biome_regions.adopt.button"));
    f.click(&lang::get("horizon.biome_regions.adopt.confirm"));
    assert!(matches!(
        &f.title.mode,
        Mode::Edit { name, adopt_regions: false, .. } if name == "Renamed"
    ));
    assert!(!adopt_offered(&f));
    assert_ne!(std::fs::read(&file).unwrap(), before);
    // The dropped region no longer asks before the world opens.
    assert_eq!(
        f.title.play(moor),
        Action::Play {
            folder: "Moor".into()
        }
    );
}
