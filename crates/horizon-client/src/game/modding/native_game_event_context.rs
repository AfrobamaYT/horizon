use super::{Game, commands::Target};
use crate::game::api_values::TryIntoEngine;

impl Game<'_> {
    pub(super) fn post_contextual_game_event(
        &mut self,
        event: &'static str,
        at: [f64; 3],
        context: mod_api::GameEventContext,
    ) -> bool {
        let state = match context
            .affected_state
            .map(|state| state.try_into_engine(&()))
            .transpose()
        {
            Ok(state) => state,
            Err(_) => return false,
        };
        let Some(entity) = context.source_entity else {
            let context = self.who_context(None, state);
            self.post_game_event(event, at, context);
            return true;
        };
        let Some(target) = self.native_entity_target(entity) else {
            return false;
        };
        if !self.native_entity_here(entity) {
            return false;
        }
        if let Target::Guest(conn) = target {
            let Some(uuid) = self.command_guest(conn).map(|guest| guest.uuid) else {
                return false;
            };
            // Capture and post inside the same guest scope: Who::Player and the
            // cause id must still identify the acting guest when listeners run.
            return self.native_as_player(uuid, |game| {
                let context = game.player_context(state);
                game.post_game_event(event, at, context);
            });
        }
        let context = match target {
            Target::Player => self.player_context(state),
            Target::Mob(id) => self.mob_context(id, state),
            _ => self.entity_event_context(entity.id(), state),
        };
        self.post_game_event(event, at, context);
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::game::api_values::TryIntoApi;
    use crate::game::native_entity_refs::tests::with_game;
    use horizon_client::{chunk::Chunk, items::ItemStack, sculk::EventSource};
    use mod_api::{EntityRef, GameEventContext};
    use std::sync::Arc;

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn sensor_receives_context_source_and_affected_state_at_requested_position() {
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            game.sculk_prepare();
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let registry = &game.pack.assets.registry;
            let sensor = registry.default_state("minecraft:sculk_sensor").unwrap();
            let stone = registry
                .default_state("minecraft:stone")
                .unwrap()
                .try_into_api()
                .unwrap();
            let wool = registry
                .default_state("minecraft:white_wool")
                .unwrap()
                .try_into_api()
                .unwrap();
            game.world.set(1, 64, 1, sensor).unwrap();
            game.sculk_sync_index();
            game.spawn_item(
                ItemStack::new("minecraft:stone", 1),
                [12.5, 64.0, 1.5],
                [0.0; 3],
                0,
            );
            let source = game.entity(game.dropped[0].id).unwrap();
            let level = game.level(game.dim.id()).unwrap();
            let at = [3.5, 64.0, 1.5];
            assert!(game.level_game_event(
                level,
                "minecraft:block_change",
                at,
                GameEventContext {
                    source_entity: Some(source),
                    affected_state: Some(stone)
                }
            ));
            let vibration = &mut game.sculk_entity((1, 64, 1)).unwrap().vibration;
            let (info, _) = vibration.selector.current.take().unwrap();
            assert_eq!(info.source, Some(EventSource::Entity(source.id())));
            assert!(game.level_game_event(
                level,
                "minecraft:block_change",
                at,
                GameEventContext {
                    source_entity: Some(source),
                    affected_state: Some(wool)
                }
            ));
            assert!(
                game.sculk_entity((1, 64, 1))
                    .unwrap()
                    .vibration
                    .selector
                    .current
                    .is_none(),
                "affected wool must dampen the vibration"
            );
            let stale = EntityRef::new(
                mod_api::WorldEpoch::new(source.epoch().get().wrapping_add(1)),
                source.id(),
            );
            assert!(!game.level_game_event(
                level,
                "minecraft:block_change",
                at,
                GameEventContext {
                    source_entity: Some(stale),
                    affected_state: None
                }
            ));
            assert!(
                game.sculk_entity((1, 64, 1))
                    .unwrap()
                    .vibration
                    .selector
                    .current
                    .is_none()
            );
            assert!(game.level_game_event(
                level,
                "minecraft:block_change",
                at,
                GameEventContext::default()
            ));
            let (info, _) = game
                .sculk_entity((1, 64, 1))
                .unwrap()
                .vibration
                .selector
                .current
                .take()
                .unwrap();
            assert_eq!(info.source, None);
        });
    }
}
