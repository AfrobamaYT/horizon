use super::tests::{TestLevel, fixture};
use super::*;
use std::cell::RefCell;

#[test]
fn block_entity_neighbor_notices_and_weak_changes() {
    struct Observer {
        inner: TestLevel,
        seen: RefCell<Vec<(BlockPos, BlockPos)>>,
        weak: bool,
    }
    impl BlockEntityNeighbors for Observer {
        fn observe_block_entity_neighbor(
            &self,
            pos: BlockPos,
            _: BlockStateId,
            neighbor: BlockPos,
        ) {
            self.seen.borrow_mut().push((pos, neighbor));
        }
        fn receives_weak_changes(&self, _: BlockPos, _: BlockStateId) -> bool {
            self.weak
        }
    }
    impl LevelAccess for Observer {
        fn block_state(&self, pos: BlockPos) -> BlockStateId {
            self.inner.block_state(pos)
        }
        fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
            self.inner.set_raw(pos, state)
        }
        fn is_loaded(&self, pos: BlockPos) -> bool {
            self.inner.is_loaded(pos)
        }
        fn random(&mut self) -> &mut LegacyRandom {
            self.inner.random()
        }
        fn game_time(&self) -> i64 {
            0
        }

        fn observe_neighbor_changed(
            &self,
            _: &mut Ticks,
            pos: BlockPos,
            _: BlockStateId,
            _: BlockId,
            _: bool,
        ) {
            self.seen.borrow_mut().push((pos, BlockPos::new(-99, 0, 0)));
        }
    }
    let Some(f) = fixture() else { return };
    let b = &f.behaviours;
    let origin = BlockPos::new(0, 64, 0);
    let adjacent = BlockPos::new(1, 64, 0);
    let far = BlockPos::new(2, 64, 0);
    for weak in [false, true] {
        let mut level = Observer {
            inner: TestLevel::new(b),
            seen: RefCell::new(Vec::new()),
            weak,
        };
        level.set_raw(
            adjacent,
            f.registry.default_state("minecraft:stone").unwrap(),
        );
        let mut ticks = Ticks::new(0);
        level.set_raw(origin, f.registry.default_state("minecraft:chest").unwrap());
        b.container_changed(&mut level, &mut ticks, origin);
        let seen = level.seen.borrow();
        assert_eq!(
            seen.iter()
                .filter(|&&(p, n)| p == adjacent && n == origin)
                .count(),
            1
        );
        assert_eq!(
            seen.iter()
                .filter(|&&(p, n)| p == far && n == origin)
                .count(),
            usize::from(weak)
        );
        drop(seen);
        level.seen.borrow_mut().clear();
        level.set_raw(
            adjacent,
            f.registry.default_state("minecraft:comparator").unwrap(),
        );
        level.set_raw(
            origin,
            f.registry.default_state("minecraft:command_block").unwrap(),
        );
        b.update_neighbour_for_output_signal(&mut level, &mut ticks, origin);
        let seen = level.seen.borrow();
        let vanilla = seen
            .iter()
            .position(|&(p, n)| p == adjacent && n == BlockPos::new(-99, 0, 0))
            .unwrap();
        let notice = seen
            .iter()
            .position(|&(p, n)| p == adjacent && n == origin)
            .unwrap();
        assert!(vanilla < notice);
        assert_eq!(
            seen.iter()
                .filter(|&&(p, n)| p == adjacent && n == origin)
                .count(),
            1
        );
    }
}

#[test]
fn block_entity_neighbor_default_is_vanilla() {
    let Some(f) = fixture() else { return };
    let b = &f.behaviours;
    let mut level = TestLevel::new(b);
    let origin = BlockPos::new(0, 64, 0);
    level.set_raw(
        origin,
        f.registry.default_state("minecraft:command_block").unwrap(),
    );
    let mut ticks = Ticks::new(0);
    b.update_neighbour_for_output_signal(&mut level, &mut ticks, origin);
    assert_eq!(
        level.block_state(origin),
        f.registry.default_state("minecraft:command_block").unwrap()
    );
    assert!(!level.receives_weak_changes(BlockPos::new(2, 64, 0), b.air()));
}
