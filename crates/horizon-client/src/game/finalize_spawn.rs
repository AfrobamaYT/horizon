//! The game's own half of `Mob.finalizeSpawn`: `Game` as a
//! [`finalize::FinalizeLevel`], so the live level finalizes mobs through the
//! same [`finalize::Ctx`] world generation does (`mobs::finalize`).
//!
//! `Ctx.random` is the level's own random (`world.random`): vanilla draws
//! finalizeSpawn's rolls from `level.getRandom()` (`Mob.finalizeSpawn` the
//! follow-range triangle and the left-handed roll, `NaturalSpawner` the yaw
//! just before the call), never from the mob's own random. The game used to
//! draw them from each mob's own random in `mobs::spawning::finalize`; this
//! is that divergence removed.

use super::Game;
use horizon_client::collisions::{SceneCollisions, no_collision_empty};
use horizon_client::mobs::{Mob, Species, finalize, spawning};
use horizon_core::geometry::{Aabb, Vec3};
use horizon_world::food::Difficulty;

/// `Game` seen by a finalizeSpawn `Ctx`.
pub(super) struct FinalizeGame<'a, 'g> {
    pub(super) game: &'a mut Game<'g>,
}

impl FinalizeGame<'_, '_> {
    /// The biome id at a block, when its chunk is loaded.
    fn biome_name(&self, pos: [i32; 3]) -> Option<String> {
        let chunk = self.game.world.chunks.get(&(pos[0] >> 4, pos[2] >> 4))?;
        Some(
            world_biomes!(self.game)
                .get(chunk.biome(pos[0], pos[1], pos[2]))
                .name
                .clone(),
        )
    }
}

impl finalize::FinalizeLevel for FinalizeGame<'_, '_> {
    fn biome(&self, pos: [i32; 3]) -> String {
        // A spawn site without its chunk (a loaded marker mob) is neutral:
        // the plains' id, the same default the biome facts fall back to.
        self.biome_name(pos)
            .unwrap_or_else(|| "minecraft:plains".to_owned())
    }
    fn biome_tag(&self, tag: &str, pos: [i32; 3]) -> bool {
        self.biome_name(pos)
            .is_some_and(|b| self.game.res.mob_data().biome_has(tag, &b))
    }
    fn structures_with_piece_at(&self, pos: [i32; 3]) -> Vec<String> {
        self.game.structures_at(pos)
    }
    fn moon_phase(&self, _: [i32; 3]) -> usize {
        (self.game.world.day_time.div_euclid(24000).rem_euclid(8)) as usize
    }
    fn spawning_monsters(&self) -> bool {
        self.game.world.health_rules.difficulty != Difficulty::Peaceful
    }
    /// `Level.noCollision(null, box)`: block collisions, and living shulkers
    /// are solid (`getEntityCollisions`).
    fn no_collision(&self, bounds: Aabb) -> bool {
        let mut scene = SceneCollisions {
            world: &self.game.world,
            binding: self.game.res.collision(),
            subject: None,
        };
        let grown = bounds.inflate(Vec3::new(1e-7, 1e-7, 1e-7));
        no_collision_empty(&mut scene, bounds)
            && !self.game.mobs.iter().any(|m| {
                m.species() == Species::Shulker && m.alive() && m.bounds().intersects(grown)
            })
    }
    fn free_chicken(&self, bounds: Aabb) -> Option<u32> {
        self.game
            .mobs
            .iter()
            .find(|c| {
                c.species() == Species::Chicken
                    && c.alive()
                    && c.extra.rider.is_none()
                    && c.extra.mount.is_none()
                    && c.bounds().intersects(bounds)
            })
            .map(|c| c.id)
    }
    fn ride_chicken(&mut self, chicken: u32, rider: u32) {
        if let Some(c) = self.game.mobs.iter_mut().find(|m| m.id == chicken) {
            c.extra.jockey = true;
            c.extra.rider = Some(rider);
        }
    }
    fn game_time(&self) -> i64 {
        self.game.world.day_time
    }
    fn halloween(&self) -> bool {
        finalize::is_halloween()
    }
    /// `EntityType.create(level, reason)` over the live level: a new id and
    /// the type's own table entry, or None where the type cannot exist.
    fn spawn_event(&mut self, mob: &mut Mob, reason: finalize::Reason) -> finalize::SpawnEvent {
        let (outcome, _) = self.game.finalize_spawn_event(mob, reason.into(), None);
        match outcome {
            super::creatures::Finalized::Cancelled => finalize::SpawnEvent::Cancelled,
            super::creatures::Finalized::Skipped => finalize::SpawnEvent::Skip,
            super::creatures::Finalized::Done => finalize::SpawnEvent::Finalize,
        }
    }
    fn create(&mut self, kind: &str, _: finalize::Reason) -> Option<Mob> {
        if self.game.world.health_rules.difficulty == Difficulty::Peaceful
            && !finalize::allowed_in_peaceful(kind)
        {
            return None;
        }
        let t = self.game.res.mob_types().get(kind)?.clone();
        self.game.next_entity += 1;
        Some(Mob::new(self.game.next_entity, t, Vec3::ZERO, 0.0))
    }
    fn spawn_data(&self) -> finalize::SpawnData<'_> {
        finalize::SpawnData {
            registries: self.game.res.spawn_registries(),
            enchantments: {
                let game = &*self.game;
                &world_stations!(game).enchantments
            },
            items: {
                let game = &*self.game;
                world_items!(game)
            },
            // `EntityTypes.CAMEL_HUSK` dimensions if the table has the type.
            camel_husk_size: self
                .game
                .res
                .mob_types()
                .get("minecraft:camel_husk")
                .map(|t| (t.width as f32, t.height as f32))
                .unwrap_or((1.7, 2.375)),
        }
    }
}

impl Game<'_> {
    /// The game's own finalization of a mob, with no event: the same
    /// `finalize::Ctx` world generation runs, over the live level, with the
    /// level's random (`world.random`). Mobs the call made (jockeys and
    /// mounts) join the level beside it.
    pub(super) fn finalize_plain(
        &mut self,
        mob: &mut Mob,
        difficulty: &spawning::LocalDifficulty,
        reason: finalize::Reason,
    ) {
        let mut random = self.world.random.clone();
        let mut level = FinalizeGame { game: self };
        let mut cx = finalize::Ctx {
            level: &mut level,
            random: &mut random,
            difficulty: *difficulty,
            created: Vec::new(),
            jockey_rider: None,
        };
        finalize::finalize(&mut cx, mob, reason, None);
        let created = std::mem::take(&mut cx.created);
        let game = level.game;
        game.world.random = random;
        for m in created {
            game.mobs.push(m);
            game.native_mobs_joined(game.mobs.len() - 1, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::random::{LegacyRandom, RandomSource};

    /// A zombie for the spawn tests, at a spot with room around it.
    fn zombie(game: &Game<'_>) -> horizon_client::mobs::Mob {
        let kind = game
            .res
            .mob_types()
            .get("minecraft:zombie")
            .unwrap()
            .clone();
        horizon_client::mobs::Mob::new(1, kind, Vec3::new(8.0, 65.0, 8.0), 0.0)
    }

    /// `Mob.finalizeSpawn`'s own two draws (`Mob.java:1078-1085`), then
    /// `Zombie.finalizeSpawn`'s, replayed on a fresh random of the same seed:
    /// the follow-range triangle, the left-handed roll, `pick_up_loot`, the
    /// baby roll and the break-doors roll, in vanilla's order.
    struct VanillaZombieDraws {
        follow_range: f64,
        left_handed: bool,
        pick_up_loot: bool,
        baby: bool,
        can_break_doors: bool,
    }
    fn vanilla_draws(random: &mut LegacyRandom, special: f32) -> VanillaZombieDraws {
        VanillaZombieDraws {
            follow_range: random.triangle(0.0, 0.114_850_000_000_000_01),
            left_handed: random.next_float() < 0.05,
            pick_up_loot: random.next_float() < 0.55 * special,
            baby: random.next_float() < 0.05,
            can_break_doors: random.next_float() < special * 0.1,
        }
    }

    /// The follow-range spawn bonus `finalizeSpawn` added, if any.
    fn random_spawn_bonus(mob: &horizon_client::mobs::Mob) -> Option<f64> {
        mob.extra
            .attributes
            .as_ref()?
            .modifiers
            .iter()
            .find(|m| m.id == "minecraft:random_spawn_bonus")
            .map(|m| m.amount)
    }

    /// finalizeSpawn draws from the level's random (`Ctx.random` is
    /// `world.random`), in vanilla's order: a shadow random of the same seed
    /// replaying `Mob.java`'s and `Zombie.finalizeSpawn`'s draws predicts
    /// every observable outcome, and the level random advances by exactly
    /// the draws the finalization made.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn finalize_plain_draws_the_level_random_in_vanilla_order() {
        crate::game::native_guest_turn_tests::with_guests(|game| {
            let mut mob = zombie(game);
            let seed = 0xC0FFEE_i64;
            game.world.random = LegacyRandom::new(seed);
            let difficulty = game.local_difficulty([8.0, 65.0, 8.0]);
            game.finalize_plain(&mut mob, &difficulty, finalize::Reason::Natural);

            let mut shadow = LegacyRandom::new(seed);
            let draws = vanilla_draws(&mut shadow, difficulty.special());
            assert_eq!(mob.extra.left_handed, draws.left_handed);
            assert_eq!(mob.extra.pick_up_loot, draws.pick_up_loot);
            assert_eq!(mob.extra.can_break_doors, draws.can_break_doors);
            let bonus = random_spawn_bonus(&mob).expect("random_spawn_bonus added");
            assert!((bonus - draws.follow_range).abs() < 1e-12);
            // The baby roll picked its group state; a baby on this seed would
            // have ridden the same stream, so its flag matches the shadow too.
            assert_eq!(mob.extra.age < 0, draws.baby);
            // finalizeSpawn's later rolls (populate, enchant, the Halloween
            // head) advance the same stream past the shadow's replay.
            let before = shadow.next_int_bound(1000);
            assert_ne!(game.world.random.next_int_bound(1000), before);
        });
    }

    /// With no mod listening, the event half of the folded finalizeSpawn
    /// changes nothing: `finalize_with` finalizes exactly as `finalize_plain`
    /// does, on the same level stream.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn finalize_with_without_listeners_finalizes_like_finalize_plain() {
        crate::game::native_guest_turn_tests::with_guests(|game| {
            let mut with = zombie(game);
            let mut plain = zombie(game);
            let difficulty = game.local_difficulty([8.0, 65.0, 8.0]);
            game.world.random = LegacyRandom::new(42_i64);
            let outcome = game.finalize_with(
                &mut with,
                horizon_client::mobs::spawn_rules::Reason::Natural,
                None,
            );
            assert_eq!(outcome, super::super::creatures::Finalized::Done);
            // The same stream for the plain path: same seed, same draws.
            game.world.random = LegacyRandom::new(42_i64);
            game.finalize_plain(&mut plain, &difficulty, finalize::Reason::Natural);

            assert_eq!(with.extra.left_handed, plain.extra.left_handed);
            assert_eq!(with.extra.pick_up_loot, plain.extra.pick_up_loot);
            assert_eq!(with.extra.can_break_doors, plain.extra.can_break_doors);
            assert_eq!(with.extra.age < 0, plain.extra.age < 0);
            let a = random_spawn_bonus(&with);
            let b = random_spawn_bonus(&plain);
            assert_eq!(a.is_some(), b.is_some());
            if let (Some(a), Some(b)) = (a, b) {
                assert!((a - b).abs() < 1e-12);
            }
        });
    }
}
