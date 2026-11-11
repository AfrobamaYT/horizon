//! Ticking eligibility through the same predicates as the server's tick phases.
use super::Game;
use mod_api::LevelRef;

impl Game<'_> {
    fn native_tick_position(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        if !self.level_in_place(level) {
            return None;
        }
        Some(
            self.level_in_world_bounds(level, pos)?
                && self.chunk(level, [pos[0] >> 4, pos[2] >> 4]).is_some(),
        )
    }

    pub(super) fn level_entity_ticking(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        Some(self.native_tick_position(level, pos)? && self.entity_ticking_area()(pos[0], pos[2]))
    }

    pub(super) fn level_should_tick_blocks_at(
        &self,
        level: LevelRef,
        pos: [i32; 3],
    ) -> Option<bool> {
        Some(self.native_tick_position(level, pos)? && self.block_ticking_area()(pos[0], pos[2]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::native_entity_refs::tests::{prepare_entity_ticking_neighborhood, with_game};
    use horizon_client::ticket_levels::Sources;
    use horizon_core::position::ChunkPos;
    use mod_api::{Authority, DimensionKey};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn ticking_reads_follow_tickets_and_pending_entities_without_loading() {
        with_game(|game| {
            let level = game.level("minecraft:overworld").unwrap();
            prepare_entity_ticking_neighborhood(game, ChunkPos::new(0, 0));
            game.tickets.update(Some(Sources {
                players: vec![(0, 0)],
                view_distance: 2,
                simulation_distance: 2,
                ..Default::default()
            }));
            let before = game.world.chunks.len();
            assert_eq!(level.entity_ticking(game, [0, 64, 0]), Some(true));
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), Some(true));
            game.world.entities_pending.insert((0, 0));
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), Some(false));
            game.world.entities_pending.remove(&(0, 0));
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), Some(true));
            let neighbour = game.world.chunks.remove(&(2, 2)).unwrap();
            assert_eq!(level.entity_ticking(game, [0, 64, 0]), Some(false));
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), Some(true));
            game.world.chunks.insert((2, 2), neighbour);
            game.tickets.update(Some(Sources::default()));
            assert_eq!(level.entity_ticking(game, [0, 64, 0]), Some(false));
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), Some(false));
            for pos in [[1_000_000, 64, 0], [0, i32::MAX, 0], [i32::MIN, 64, 0]] {
                assert_eq!(level.entity_ticking(game, pos), Some(false));
                assert_eq!(level.should_tick_blocks_at(game, pos), Some(false));
            }
            assert_eq!(game.world.chunks.len(), before);
            let unopened = LevelRef::new(level.epoch(), DimensionKey::NETHER);
            assert_eq!(unopened.entity_ticking(game, [0, 64, 0]), None);
            game.enter_level(crate::game::Dim::Nether).unwrap();
            assert_eq!(level.entity_ticking(game, [0, 64, 0]), None);
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), None);
            game.native_lifecycle
                .begin_open(Authority::IntegratedServer);
            assert_eq!(level.entity_ticking(game, [0, 64, 0]), None);
            game.options.execution_role = crate::game::execution_role::ExecutionRole::Client;
            assert_eq!(level.should_tick_blocks_at(game, [0, 64, 0]), None);
        });
    }
}
