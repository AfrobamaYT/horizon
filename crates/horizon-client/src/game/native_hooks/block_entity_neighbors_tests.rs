#![cfg(test)]
use crate::game::native_entity_refs::tests::with_game;
use horizon_client::chunk::Chunk;
use horizon_core::position::BlockPos;
use mod_api::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

type Notices = Arc<Mutex<Vec<(BlockPos, BlockPos)>>>;
struct Counter(Notices);
#[mod_api::horizon_part]
impl BlockPart for Counter {
    fn on_neighbor_change(&self, _: &dyn LevelHost, event: &mut OnNeighborChangeEvent) {
        self.0.lock().unwrap().push((event.pos, event.neighbor));
    }
}
struct Weak(Arc<AtomicBool>, Arc<AtomicUsize>);
#[mod_api::horizon_part]
impl BlockPart for Weak {
    fn weak_changes(&self, _: &dyn LevelHost, event: &mut WeakChangesEvent) {
        self.1.fetch_add(1, Ordering::SeqCst);
        event.receive = self.0.load(Ordering::SeqCst);
    }
}
struct Listener(Notices, Arc<AtomicBool>, Arc<AtomicUsize>);
#[mod_api::__subscriptions]
impl Hooks for Listener {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let glass = r.block_key("minecraft:glass");
        r.block_part(glass, Counter(self.0.clone()));
        r.block_part(glass, Weak(self.1.clone(), self.2.clone()));
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn installed_block_entity_neighbor_parts_dispatch_through_game() {
    let notices = Arc::new(Mutex::new(Vec::new()));
    let weak = Arc::new(AtomicBool::new(true));
    let queries = Arc::new(AtomicUsize::new(0));
    let mods: Mods = vec![ModEntry::new(
        "neighbor_counter",
        Box::new(Listener(notices.clone(), weak.clone(), queries.clone())),
    )]
    .into();
    mod_api::testing::with_mods(mods.clone(), || {
        mod_api::__private::part_tests::install_parts_fixture(&mods).unwrap();
        with_game(|game| {
            let directory = tempfile::tempdir().unwrap();
            game.save = Arc::new(horizon_client::save::SaveDir::new(
                directory.path().to_path_buf(),
                horizon_client::save::LevelShape::OVERWORLD,
            ));
            game.options.dedicated = Some(Vec::new());
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let registry = game.pack.assets.registry;
            let chest = registry.default_state("minecraft:chest").unwrap();
            let glass = registry.default_state("minecraft:glass").unwrap();
            let stone = registry.default_state("minecraft:stone").unwrap();
            let source = BlockPos::new(4, 64, 4);
            let adjacent = BlockPos::new(3, 64, 4);
            let far = BlockPos::new(6, 64, 4);
            let bare = BlockPos::new(4, 64, 3);
            for (pos, state) in [
                (source, chest),
                (adjacent, glass),
                (BlockPos::new(5, 64, 4), stone),
                (far, glass),
                (bare, stone),
            ] {
                game.world.set(pos.x, pos.y, pos.z, state).unwrap();
            }
            for enabled in [true, false] {
                weak.store(enabled, Ordering::SeqCst);
                queries.store(0, Ordering::SeqCst);
                notices.lock().unwrap().clear();
                game.container_changed((source.x, source.y, source.z));
                let heard = notices.lock().unwrap();
                assert_eq!(
                    heard
                        .iter()
                        .filter(|&&(p, n)| p == adjacent && n == source)
                        .count(),
                    1
                );
                assert_eq!(
                    heard
                        .iter()
                        .filter(|&&(p, n)| p == far && n == source)
                        .count(),
                    usize::from(enabled)
                );
                assert_eq!(heard.iter().filter(|&&(p, _)| p == bare).count(), 0);
                assert_eq!(heard.len(), if enabled { 2 } else { 1 });
                assert_eq!(queries.load(Ordering::SeqCst), 1);
            }
        });
    });
}
