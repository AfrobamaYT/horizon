//! The table a server publishes for its world generation, as the worldgen Part methods read it on
//! threads with no game. One test: the published table is process-wide.
use super::*;
use crate::*;
use std::sync::Mutex;

type Heard = Arc<Mutex<Vec<&'static str>>>;

/// Answers nothing, recording that the part attached by the block's key was asked.
struct KeySoil(Heard);
#[mod_api::horizon_part]
impl BlockPart for KeySoil {
    fn on_tree_grow(
        &self,
        _: &dyn WorldgenHost,
        _: &mut OnTreeGrowEvent<'_>,
    ) -> Option<Option<BlockState>> {
        self.0.lock().unwrap().push("key");
        None
    }
}
/// The same, for the part attached through the block's tag.
struct TagSoil(Heard);
#[mod_api::horizon_part]
impl BlockPart for TagSoil {
    fn on_tree_grow(
        &self,
        _: &dyn WorldgenHost,
        _: &mut OnTreeGrowEvent<'_>,
    ) -> Option<Option<BlockState>> {
        self.0.lock().unwrap().push("tag");
        None
    }
}
/// A tag part on a hook without the worldgen context, which only a game thread asks.
struct Comparator;
#[mod_api::horizon_part]
impl BlockPart for Comparator {
    fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output += 1;
    }
}
struct Soils(Heard);
#[mod_api::__subscriptions]
impl Hooks for Soils {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let grass = r.block_key("minecraft:grass_block");
        r.block_part(grass, KeySoil(self.0.clone()));
        r.block_part_tag("minecraft:dirt", TagSoil(self.0.clone()));
        r.block_part_tag("minecraft:dirt", Comparator);
    }
}

struct Draws;
impl StructureRandom for Draws {
    fn next_int_bound(&mut self, _: i32) -> i32 {
        0
    }
    fn next_long(&mut self) -> i64 {
        0
    }
    fn next_float(&mut self) -> f32 {
        0.0
    }
    fn next_double(&mut self) -> f64 {
        0.0
    }
    fn next_boolean(&mut self) -> bool {
        false
    }
}

#[test]
fn a_thread_with_no_game_asks_the_published_key_and_tag_parts_and_a_feature_keeps_its_table() {
    let heard: Heard = Arc::default();
    let mods: Mods = vec![ModEntry::new("soils", Box::new(Soils(heard.clone())))].into();
    crate::install(mods.clone());
    install_parts(&mods, &crate::parts::tests::rule_schema(), |_| false).unwrap();
    crate::content::registry_keys::resolve_block_keys(
        ["minecraft:air", "minecraft:grass_block"],
        |_| {
            horizon_mod_runtime::block_definitions::BlockDefinition::new(&[&Default::default()], 0)
                .map_err(|error| format!("{error:?}"))
        },
    )
    .unwrap();
    let prepare = |members: &[&str]| {
        let tags = HashMap::from([(
            "minecraft:dirt".to_owned(),
            members.iter().map(|member| member.to_string()).collect(),
        )]);
        let known = KnownIds {
            block: &|_| true,
            item: &|_| false,
        };
        prepare_parts(&tags, &HashMap::new(), known).unwrap()
    };
    let (tagged, untagged) = (prepare(&["minecraft:grass_block"]), prepare(&[]));
    let grass = BlockState::from(BlockStateRecord::new(
        crate::content::registry_keys::block_key("minecraft:grass_block").unwrap(),
        0,
    ));
    let (installed_mods, installed_parts) = (crate::TEST_MODS.get(), TEST_PARTS.get());
    // A worldgen worker: this test's mods and parts, and no game.
    std::thread::spawn(move || {
        crate::TEST_MODS.set(installed_mods);
        TEST_PARTS.set(installed_parts);
        let asked = || std::mem::take(&mut *heard.lock().unwrap());
        let grow = || {
            let mut soil = None;
            crate::world::ground::native_on_tree_grow(
                &OwningWorldgen::new(&|_| None, &|_, _| None),
                grass,
                [0, 63, 0],
                &mut Draws,
                &mut soil,
            );
            assert_eq!(soil, None);
        };
        publish_worldgen_parts(tagged);
        grow();
        assert_eq!(asked(), ["key", "tag"]);
        // The game's table, which a tag part needs, is not on this thread.
        assert!(try_parts(HookId::AnalogOutput, "analog_output").is_err());
        worldgen_feature(|| {
            // A `/reload` drops the block from the tag while the feature runs.
            publish_worldgen_parts(untagged);
            grow();
            assert_eq!(asked(), ["key", "tag"]);
        });
        grow();
        assert_eq!(asked(), ["key"]);
    })
    .join()
    .unwrap();
}
