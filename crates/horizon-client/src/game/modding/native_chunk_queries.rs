//! Read-only chunk facts that never request residency.
use super::Game;
use horizon_core::random::{RandomSource, slime_chunk_random};
use mod_api::ChunkRef;

impl Game<'_> {
    pub(super) fn chunk_is_slime_chunk(&self, chunk: ChunkRef) -> Option<bool> {
        let seed = self.level_seed(chunk.level())?;
        let [x, z] = chunk.pos();
        // Reuse the same world-bound validation as handles for unloaded chunks.
        self.level_chunk_ref(chunk.level(), [x, z])?;
        Some(
            slime_chunk_random(x, z, seed, 987_234_911)
                .next_int_bound(10)
                .unwrap_or(1)
                == 0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::native_entity_refs::tests::with_game;
    use mod_api::{Authority, ChunkRef, DimensionKey, LevelRef};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn slime_queries_work_without_loading_and_refuse_stale_handles() {
        with_game(|game| {
            let level = game.level("minecraft:overworld").unwrap();
            let before = game.world.chunks.len();
            let seed = level.seed(game).unwrap();
            for pos in [[0, 0], [-17, 29], [100_000, -100_000]] {
                let chunk = level.chunk_ref(game, pos).unwrap();
                let expected = slime_chunk_random(pos[0], pos[1], seed, 987_234_911)
                    .next_int_bound(10)
                    .unwrap()
                    == 0;
                assert_eq!(chunk.is_slime_chunk(game), Some(expected));
                assert_eq!(chunk.is_slime_chunk(game), Some(expected));
            }
            assert_eq!(game.world.chunks.len(), before);
            let invalid = ChunkRef::new(level, [i32::MAX, 0]);
            assert_eq!(invalid.is_slime_chunk(game), None);
            let unopened = LevelRef::new(level.epoch(), DimensionKey::NETHER);
            assert_eq!(ChunkRef::new(unopened, [0, 0]).is_slime_chunk(game), None);
            game.enter_level(crate::game::Dim::Nether).unwrap();
            let parked = ChunkRef::new(level, [-17, 29]);
            assert_eq!(
                parked.is_slime_chunk(game),
                Some(
                    slime_chunk_random(-17, 29, seed, 987_234_911)
                        .next_int_bound(10)
                        .unwrap()
                        == 0
                )
            );
            game.native_lifecycle
                .begin_open(Authority::IntegratedServer);
            assert_eq!(ChunkRef::new(level, [0, 0]).is_slime_chunk(game), None);
            game.options.execution_role = crate::game::execution_role::ExecutionRole::Client;
            assert_eq!(parked.is_slime_chunk(game), None);
        });
    }
}
