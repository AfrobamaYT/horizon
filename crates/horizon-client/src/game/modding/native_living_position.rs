//! `LivingRef` line of sight and random teleport (#944): the engine's own
//! clip and `randomTeleport` paths, for players and mobs alike.
use super::{Game, commands::Target};
use horizon_core::geometry::Vec3;
use mod_api::{EntityRef, LivingRef};

impl Game<'_> {
    /// `LivingEntity.hasLineOfSight(Entity)`: whether nothing that collides
    /// (`ClipContext.Block.COLLIDER`, fluids ignored) lies between the
    /// living entity's eyes and `target`'s eye height, within 128 blocks.
    /// False for a target in another level. None for a stale or unavailable
    /// living entity or target, one outside the level in place, or on a
    /// client.
    pub(super) fn native_living_has_line_of_sight(
        &self,
        living: LivingRef,
        target: EntityRef,
    ) -> Option<bool> {
        let viewer = self.native_living_target(living)?;
        let seen = self.native_entity_target(target)?;
        if self.target_level(viewer) != self.dim {
            return None;
        }
        if self.target_level(seen) != self.dim {
            return Some(false);
        }
        let feet = self.native_entity_position(living.entity())?;
        let from = self.native_entity_eye_position(living.entity())?;
        let to = self.native_entity_eye_position(target)?;
        let w = self.mob_world_base(&[]);
        Some(w.line_of_sight_for(
            Some(feet[1]),
            Vec3::new(from[0], from[1], from[2]),
            Vec3::new(to[0], to[1], to[2]),
        ))
    }

    /// `LivingEntity.randomTeleport(x, y, z, true, avoid)`: the point `at`
    /// is clamped into the world border and drops to the first block in
    /// `minecraft:entities_can_teleport_to` below it, losing a block of height
    /// per step down. The entity lands there, keeping its motion, unless that
    /// ground or a block in its box at the landing is in the `avoid` block
    /// tag, the box collides or holds a liquid, or the species refuses the
    /// spot (an enderman never lands on ground holding water). A landing
    /// stops a mob's navigation and broadcasts entity event 46 (the portal
    /// particles). Nothing else is played: a chorus fruit's sound and an
    /// enderman's own teleport effects belong to their callers.
    ///
    /// True when the entity moved. False when no spot was found, for a
    /// non-finite point, a fake player, a player in another level, an armor
    /// stand or the dragon, a stale handle, or on a client.
    pub(super) fn native_living_random_teleport(
        &mut self,
        living: LivingRef,
        at: [f64; 3],
        avoid: Option<&str>,
    ) -> bool {
        if !at.iter().all(|v| v.is_finite()) {
            return false;
        }
        let at = Vec3::new(at[0], at[1], at[2]);
        match self.native_living_target(living) {
            Some(target @ (Target::Player | Target::Guest(_))) => {
                if self.target_level(target) != self.dim {
                    return false;
                }
                self.native_player_sleep_write(target, |game| {
                    game.player.is_some() && game.random_teleport(at, avoid)
                })
            }
            Some(Target::Mob(id)) => {
                if !self.mobs.iter().any(|m| m.id == id && !m.removed) {
                    return false;
                }
                let mut mobs = std::mem::take(&mut self.mobs);
                let w = self.mob_world_base(&[]);
                let mut events = Vec::new();
                let moved = mobs.iter_mut().find(|m| m.id == id).is_some_and(|m| {
                    horizon_client::mobs::living::random_teleport(m, &w, at, avoid, &mut events)
                });
                self.mobs = mobs;
                let _ = self.handle_mob_events(events);
                moved
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use super::*;
    use horizon_client::mobs::Mob;
    use mod_api::PathType;

    /// Lit empty chunks around the origin, a stone floor at y 63 over
    /// -8..8, and two zombies standing on it five blocks apart.
    fn floor_and_zombies(game: &mut Game<'_>) {
        for (x, z) in (-1..=1).flat_map(|x| (-1..=1).map(move |z| (x, z))) {
            let mut chunk = horizon_client::chunk::Chunk::empty(x, z);
            chunk.lit = true;
            game.world.chunks.insert((x, z), std::sync::Arc::new(chunk));
        }
        let stone = game
            .pack
            .assets
            .registry
            .default_state("minecraft:stone")
            .unwrap();
        for x in -8..8 {
            for z in -8..8 {
                game.world.set(x, 63, z, stone).unwrap();
            }
        }
        let zombie = game
            .res
            .mob_types()
            .get("minecraft:zombie")
            .unwrap()
            .clone();
        game.mobs
            .push(Mob::new(12, zombie.clone(), Vec3::new(0.5, 64.0, 0.5), 0.0));
        game.mobs
            .push(Mob::new(13, zombie, Vec3::new(5.5, 64.0, 0.5), 0.0));
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn line_of_sight_follows_the_blocks_between() {
        with_game(|game| {
            floor_and_zombies(game);
            let a = game.native_living(game.native_entity(12).unwrap()).unwrap();
            let b = game.native_entity(13).unwrap();
            assert_eq!(a.has_line_of_sight(game, b), Some(true));
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            for y in 64..67 {
                game.world.set(3, y, 0, stone).unwrap();
            }
            assert_eq!(
                a.has_line_of_sight(game, b),
                Some(false),
                "a wall in between"
            );
            game.mobs.retain(|m| m.id != 13);
            assert_eq!(a.has_line_of_sight(game, b), None, "a gone target");
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn random_teleport_drops_to_the_ground_and_honours_the_avoid_tag() {
        with_game(|game| {
            floor_and_zombies(game);
            let zombie = game.native_living(game.native_entity(12).unwrap()).unwrap();
            let at = |game: &Game<'_>| game.mobs.iter().find(|m| m.id == 12).unwrap().position;
            // From six blocks up it lands on the floor, one block lower per
            // step down (`randomTeleport`).
            assert!(zombie.random_teleport(game, [4.5, 70.0, 4.5], None));
            assert_eq!(at(game), Vec3::new(4.5, 64.0, 4.5));
            assert_eq!(
                game.mobs.iter().find(|m| m.id == 12).unwrap().previous,
                at(game)
            );
            // No ground below (outside the floor): it stays.
            assert!(!zombie.random_teleport(game, [12.5, 70.0, 12.5], None));
            assert_eq!(at(game), Vec3::new(4.5, 64.0, 4.5));
            // Ground in the avoid tag: bedrock is in the enderman's.
            let bedrock = game
                .pack
                .assets
                .registry
                .default_state("minecraft:bedrock")
                .unwrap();
            game.world.set(-4, 63, -4, bedrock).unwrap();
            assert!(!zombie.random_teleport(
                game,
                [-3.5, 66.0, -3.5],
                Some("minecraft:enderman_does_not_teleport_to"),
            ));
            assert!(zombie.random_teleport(game, [-3.5, 66.0, -3.5], None));
            assert_eq!(at(game), Vec3::new(-3.5, 64.0, -3.5));
            assert!(!zombie.random_teleport(game, [f64::NAN, 66.0, 0.5], None));
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn no_action_time_and_pathfinding_malus_reach_the_mob() {
        with_game(|game| {
            floor_and_zombies(game);
            let mob = game
                .native_mob_ref(game.native_entity(12).unwrap())
                .unwrap();
            assert_eq!(mob.no_action_time(game), Some(0));
            assert!(mob.set_no_action_time(game, 650));
            assert_eq!(
                game.mobs
                    .iter()
                    .find(|m| m.id == 12)
                    .unwrap()
                    .ai
                    .no_action_time,
                650
            );
            assert_eq!(mob.no_action_time(game), Some(650));

            // `noActionTime` is a Java `int`: the tick past `i32::MAX` wraps
            // instead of panicking. Kept so it cannot despawn meanwhile.
            assert!(mob.set_persistence_required(game));
            assert!(mob.set_no_action_time(game, i32::MAX));
            game.tick_mobs().unwrap();
            let after = mob.no_action_time(game).unwrap();
            assert!(after < 0, "wrapped past i32::MAX, got {after}");
            assert!(mob.set_no_action_time(game, 650));

            assert_eq!(mob.pathfinding_malus(game, PathType::Water), Some(8.0));
            assert!(mob.set_pathfinding_malus(game, PathType::Water, -1.0));
            assert_eq!(mob.pathfinding_malus(game, PathType::Water), Some(-1.0));
            assert_eq!(
                game.mobs
                    .iter()
                    .find(|m| m.id == 12)
                    .unwrap()
                    .malus(PathType::Water),
                -1.0,
                "the pathfinder reads the same override"
            );
            assert_eq!(mob.pathfinding_malus(game, PathType::Lava), Some(-1.0));
            assert!(!mob.set_pathfinding_malus(game, PathType::Water, f32::NAN));

            game.mobs.retain(|m| m.id != 12);
            assert_eq!(mob.no_action_time(game), None);
            assert!(!mob.set_no_action_time(game, 1));
            assert_eq!(mob.pathfinding_malus(game, PathType::Water), None);
        });
    }
}
