//! `explosion_detonate`: the blocks and entities a server explosion is about to
//! reach (NeoForge's `ExplosionEvent.Detonate`), and the entities a mod spares.
use super::Game;

impl Game<'_> {
    /// `ServerExplosion.explode` posting `onExplosionDetonate` before any block or
    /// entity is hit. The entities are `level.getEntities(source, box)`, the box
    /// the centre ± (2 × radius + 1) floored, without the exploding source; they
    /// are listed only when a mod listens. Returns the listed entities the mods
    /// took out: the blast's walks skip them, so they are neither hurt nor pushed.
    pub(super) fn explosion_detonate(
        &mut self,
        center: [f64; 3],
        radius: f32,
        blocks: &mut Vec<[i32; 3]>,
    ) -> Vec<u32> {
        let dimension = self.dim.id();
        if !mod_api::__private::listened(mod_api::HookId::ExplosionDetonate) {
            return Vec::new();
        }
        let reach = f64::from(radius) * 2.0 + 1.0;
        let min = center.map(|c| (c - reach).floor());
        let max = center.map(|c| (c + reach).floor());
        let source = self
            .combat
            .explosion_direct
            .or(self.combat.explosion_source);
        let mut entities = self
            .level(dimension)
            .and_then(|level| self.level_entities_in(level, min, max))
            .unwrap_or_default();
        entities.retain(|entity| Some(entity.id()) != source);
        let mut spared: Vec<u32> = entities.iter().map(|entity| entity.id()).collect();
        self.native_explosion_detonate(dimension, center, blocks, &mut entities);
        spared.retain(|id| !entities.iter().any(|entity| entity.id() == *id));
        spared
    }
}

#[cfg(test)]
mod tests {
    use crate::game::native_entity_refs::tests::with_game;
    use horizon_client::mobs::Mob;
    use horizon_core::geometry::Vec3;
    use mod_api::{ExplosionDetonateEvent, Hooks, ServerHost};
    use std::sync::{Arc, Mutex};

    const SPARED: u32 = 901;

    struct Probe(Arc<Mutex<Vec<u32>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Probe {
        fn explosion_detonate(
            &self,
            _: &mut dyn ServerHost,
            event: &mut ExplosionDetonateEvent<'_>,
        ) {
            *self.0.lock().unwrap() = event.entities.iter().map(|e| e.id()).collect();
            event.entities.retain(|e| e.id() != SPARED);
        }
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn an_entity_taken_out_is_neither_hurt_nor_pushed() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
            "detonate",
            Box::new(Probe(heard.clone())),
        )]
        .into();
        mod_platform::testing::with_mods(mods, || {
            with_game(|game| {
                let zombie = game
                    .res
                    .mob_types()
                    .get("minecraft:zombie")
                    .unwrap()
                    .clone();
                for (id, x) in [(900, 2.0), (SPARED, -2.0)] {
                    game.mobs
                        .push(Mob::new(id, zombie.clone(), Vec3::new(x, 1.0, 0.0), 0.0));
                }
                game.explode(
                    None,
                    [0.0, 1.0, 0.0],
                    3.0,
                    super::super::explosion::ExplosionSource::default(),
                )
                .unwrap();
                let mob = |id| game.mobs.iter().find(|m| m.id == id).unwrap();
                let (hit, spared) = (mob(900), mob(SPARED));
                assert!(hit.health < 20.0);
                assert_ne!(hit.velocity, Vec3::ZERO);
                assert_eq!(spared.health, 20.0);
                assert_eq!(spared.velocity, Vec3::ZERO);
            });
        });
        let heard = heard.lock().unwrap();
        assert!(heard.contains(&900) && heard.contains(&SPARED), "{heard:?}");
    }
}
