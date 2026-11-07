//! An entity's invulnerability for native server mods, through an
//! `EntityRef` (docs/modding/src/how-to/native-mods.md, "Entity and level handles"): the
//! `Invulnerable` flag (`Entity.permanentlyInvulnerable`) and
//! `LivingEntity.damageCooldownTime`, the ticks after a hit in which a
//! weaker one does only the difference. Horizon has no counterpart yet of
//! 26.3's own `Entity.invulnerableTime` timer, so neither is offered. Reads
//! return None and writes false for a stale handle, on a client, and for a
//! kind Horizon keeps no such state for.
use super::{Game, commands::Target};
use mod_api::EntityRef;

impl Game<'_> {
    /// A player's body, where it is kept now.
    fn native_player_body(&self, t: Target) -> Option<&horizon_client::motion::PlayerMotion> {
        match t {
            Target::Player => self.player.as_ref(),
            Target::Guest(conn) => self.lan.as_ref()?.hub.guest(conn)?.motion.as_deref(),
            _ => None,
        }
    }
    fn native_player_body_mut(
        &mut self,
        t: Target,
    ) -> Option<&mut horizon_client::motion::PlayerMotion> {
        match t {
            Target::Player => self.player.as_mut(),
            Target::Guest(conn) => self.lan.as_mut()?.hub.guests.motion_mut(conn),
            _ => None,
        }
    }

    /// `Entity.isPermanentlyInvulnerable`: the `Invulnerable` flag, which
    /// only damage that bypasses invulnerability (the void, `/kill`) or a
    /// creative player gets past. Mobs keep it on themselves (saved with
    /// them), players on their body (#916, not saved: Horizon's player save
    /// keeps no such field yet), the other kinds in the saved `Invulnerable`
    /// tag of their entity data (#916) — the tag the item damage path
    /// already refuses by.
    pub(super) fn entity_permanently_invulnerable(&self, entity: EntityRef) -> Option<bool> {
        match self.native_entity_target(entity)? {
            Target::Mob(id) => Some(self.mobs.iter().find(|m| m.id == id)?.invulnerable_flag),
            t @ (Target::Player | Target::Guest(_)) => {
                Some(self.native_player_body(t)?.health.permanently_invulnerable)
            }
            t => Some(self.saved_invulnerable_flag(t)?),
        }
    }
    /// `Entity.setInvulnerable`: the write side of
    /// [`entity_permanently_invulnerable`](Self::entity_permanently_invulnerable):
    /// a mob's flag (saved with it), a player's body, the saved tag for the
    /// other kinds. False where the entity is gone.
    pub(super) fn entity_set_invulnerable(
        &mut self,
        entity: EntityRef,
        invulnerable: bool,
    ) -> bool {
        match self.native_entity_target(entity) {
            Some(Target::Mob(id)) => match self.mobs.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.invulnerable_flag = invulnerable;
                    true
                }
                None => false,
            },
            Some(t @ (Target::Player | Target::Guest(_))) => {
                self.native_player_body_mut(t).is_some_and(|p| {
                    p.health.permanently_invulnerable = invulnerable;
                    true
                })
            }
            Some(t) => self.set_saved_invulnerable_flag(t, invulnerable),
            None => false,
        }
    }

    /// The saved `Invulnerable` tag of a non-mob entity's data (#916):
    /// `false` where its entity data has none. None for a kind that keeps
    /// none (a frame, a painting, a fang).
    fn saved_invulnerable_flag(&self, t: Target) -> Option<bool> {
        let id = match t {
            Target::Stand(id) | Target::Dragon(id) | Target::Item(id) => id,
            _ => return None,
        };
        Some(self.saved_invulnerability(id).0)
    }
    /// Writes the `Invulnerable` tag into a non-mob entity's saved data
    /// (#916), the compound its own save builds on.
    fn set_saved_invulnerable_flag(&mut self, t: Target, invulnerable: bool) -> bool {
        use horizon_core::nbt::Tag;
        let id = match t {
            Target::Stand(id) | Target::Dragon(id) | Target::Item(id) => id,
            _ => return false,
        };
        let mut compound = match self.persist.raw.get(&id) {
            Some(Tag::Compound(map)) => map.clone(),
            _ => std::collections::BTreeMap::new(),
        };
        compound.insert("Invulnerable".into(), Tag::Byte(i8::from(invulnerable)));
        self.persist.raw.insert(id, Tag::Compound(compound));
        true
    }

    /// `Entity.invulnerableTime` (#916): the entity-level invulnerability
    /// window, its own field on a mob and a player — never the living damage
    /// cooldown — and the saved `invulnerable_time` tag for the other kinds.
    /// Counted down by the base tick; `isTemporarilyInvulnerable` is it above
    /// zero. None for a stale handle or a kind that keeps none.
    pub(super) fn entity_invulnerable_time(&self, entity: EntityRef) -> Option<i32> {
        match self.native_entity_target(entity)? {
            Target::Mob(id) => Some(self.mobs.iter().find(|m| m.id == id)?.invulnerable_time),
            t @ (Target::Player | Target::Guest(_)) => {
                Some(self.native_player_body(t)?.health.invulnerable_time)
            }
            t => self.saved_invulnerable_time(t),
        }
    }
    /// `Entity.setInvulnerableTime` (#916): the write side of
    /// [`native_entity_invulnerable_time`](Self::native_entity_invulnerable_time).
    /// False where the entity is gone or keeps no such field.
    pub(super) fn entity_set_invulnerable_time(&mut self, entity: EntityRef, ticks: i32) -> bool {
        let ticks = ticks.max(0);
        match self.native_entity_target(entity) {
            Some(Target::Mob(id)) => match self.mobs.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.invulnerable_time = ticks;
                    true
                }
                None => false,
            },
            Some(t @ (Target::Player | Target::Guest(_))) => {
                self.native_player_body_mut(t).is_some_and(|p| {
                    p.health.invulnerable_time = ticks;
                    true
                })
            }
            Some(t) => self.set_saved_invulnerable_time(t, ticks),
            None => false,
        }
    }

    /// A non-mob kind's saved invulnerability (#916), the one parse of its
    /// entity data: the `Invulnerable` flag and the `invulnerable_time`
    /// window. `(false, 0)` where its data has neither.
    fn saved_invulnerability(&self, id: u32) -> (bool, i32) {
        use horizon_core::nbt::Tag;
        match self.persist.raw.get(&id) {
            Some(Tag::Compound(map)) => (
                matches!(map.get(&"Invulnerable".into()), Some(Tag::Byte(1))),
                match map.get(&"invulnerable_time".into()) {
                    Some(Tag::Int(ticks)) => *ticks,
                    _ => 0,
                },
            ),
            _ => (false, 0),
        }
    }
    /// The saved `invulnerable_time` tag of a non-mob entity's data (#916):
    /// 0 where its entity data has none. None for a kind that keeps none.
    fn saved_invulnerable_time(&self, t: Target) -> Option<i32> {
        let id = match t {
            Target::Stand(id) | Target::Dragon(id) | Target::Item(id) => id,
            _ => return None,
        };
        Some(self.saved_invulnerability(id).1)
    }
    /// `Entity.isInvulnerableToBase` over a non-mob kind's saved flag and
    /// window (#916) — the one place that refusal is decided: past only for
    /// `#bypasses_invulnerability` damage and a creative player's hit
    /// (`isCreativePlayer`). False where the saved state has neither.
    pub(in crate::game) fn saved_refuses_damage(
        &self,
        id: u32,
        damage_type: &str,
        creative_source: bool,
    ) -> bool {
        let (flag, window) = self.saved_invulnerability(id);
        if !flag && window <= 0 {
            return false;
        }
        let registry = &self.health_resources.damage;
        let bypass = registry
            .id(damage_type)
            .is_some_and(|kind| registry.in_tag(kind, "minecraft:bypasses_invulnerability"));
        !(bypass || creative_source)
    }
    /// Writes the `invulnerable_time` tag into a non-mob entity's saved data
    /// (#916).
    fn set_saved_invulnerable_time(&mut self, t: Target, ticks: i32) -> bool {
        use horizon_core::nbt::Tag;
        let id = match t {
            Target::Stand(id) | Target::Dragon(id) | Target::Item(id) => id,
            _ => return false,
        };
        let mut compound = match self.persist.raw.get(&id) {
            Some(Tag::Compound(map)) => map.clone(),
            _ => std::collections::BTreeMap::new(),
        };
        compound.insert("invulnerable_time".into(), Tag::Int(ticks.max(0)));
        self.persist.raw.insert(id, Tag::Compound(compound));
        true
    }
    /// `Entity.baseTick`'s countdown of a non-mob kind's saved
    /// `invulnerable_time` (#916): one less each tick while it runs, so the
    /// window an entity carries in its saved data closes like a mob's and a
    /// player's do.
    pub(in crate::game) fn tick_saved_invulnerable_time(&mut self, id: u32) {
        use horizon_core::nbt::Tag;
        let ticks = self.saved_invulnerability(id).1;
        if ticks <= 0 {
            return;
        }
        let ticks = ticks - 1;
        let mut compound = match self.persist.raw.get(&id) {
            Some(Tag::Compound(map)) => map.clone(),
            _ => std::collections::BTreeMap::new(),
        };
        compound.insert("invulnerable_time".into(), Tag::Int(ticks));
        self.persist.raw.insert(id, Tag::Compound(compound));
    }

    /// `LivingEntity.damageCooldownTime`: 20 after a hit, counting down;
    /// while it is above 10 a hit does only what it deals beyond the last
    /// one. Mobs and players.
    pub(super) fn entity_damage_cooldown_time(&self, entity: EntityRef) -> Option<i32> {
        match self.native_entity_target(entity)? {
            Target::Mob(id) => Some(self.mobs.iter().find(|m| m.id == id)?.invulnerable),
            t => Some(self.native_player_body(t)?.health.hurt.cooldown),
        }
    }
    /// Sets `LivingEntity.damageCooldownTime`, as mods do to let rapid hits
    /// through (0). Mobs and players.
    pub(super) fn entity_set_damage_cooldown_time(
        &mut self,
        entity: EntityRef,
        ticks: i32,
    ) -> bool {
        let Some(t) = self.native_entity_target(entity) else {
            return false;
        };
        match t {
            Target::Mob(id) => match self.mobs.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.invulnerable = ticks;
                    true
                }
                None => false,
            },
            t => match self.native_player_body_mut(t) {
                Some(p) => {
                    p.health.hurt.cooldown = ticks;
                    true
                }
                None => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use horizon_client::mobs::Mob;
    use horizon_core::geometry::Vec3;
    use mod_api::{EntityRef, NativeWorldEpoch};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_mob_keeps_its_flag_and_damage_cooldown() {
        with_game(|game| {
            let zombie = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs.push(Mob::new(12, zombie, Vec3::ZERO, 0.0));
            let mob = game.entity(12).unwrap();
            assert_eq!(game.entity_permanently_invulnerable(mob), Some(false));
            assert!(game.entity_set_invulnerable(mob, true));
            assert_eq!(game.entity_permanently_invulnerable(mob), Some(true));
            assert!(game.mobs[0].invulnerable_flag);
            assert_eq!(game.entity_damage_cooldown_time(mob), Some(0));
            assert!(game.entity_set_damage_cooldown_time(mob, 20));
            assert_eq!(game.mobs[0].invulnerable, 20);
            assert_eq!(game.entity_damage_cooldown_time(mob), Some(20));
            let stale = EntityRef::new(NativeWorldEpoch::new(mob.epoch().get() + 1), 12);
            assert_eq!(game.entity_permanently_invulnerable(stale), None);
            assert!(!game.entity_set_invulnerable(stale, false));
            assert_eq!(game.entity_damage_cooldown_time(stale), None);
            assert!(!game.entity_set_damage_cooldown_time(stale, 0));
            assert!(game.mobs[0].invulnerable_flag);
        });
    }

    /// #916: the permanent flag reaches players and non-mob kinds, and a
    /// player's damage is really refused by it.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_flag_reaches_players_and_non_mobs_and_refuses_player_damage() {
        use horizon_client::motion::PlayerMotion;
        super::super::native_entity_refs::tests::with_game(|game| {
            game.player = Some(PlayerMotion::new(game.motion, Vec3::ZERO).unwrap());
            game.lan_acting_uuid = Some(123);
            game.lan_acting_entity = Some(20);
            let player = game.entity(20).unwrap();
            game.armor_stands
                .push(horizon_client::entities::ArmorStand::new(
                    14,
                    Vec3::ZERO,
                    0.0,
                ));
            let stand = game.entity(14).unwrap();

            // A player's flag, on their body, refusing their damage — and
            // `#bypasses_invulnerability` still getting past.
            assert_eq!(game.entity_permanently_invulnerable(player), Some(false));
            assert!(game.entity_set_invulnerable(player, true));
            assert_eq!(game.entity_permanently_invulnerable(player), Some(true));
            game.hurt_player_by("minecraft:generic", 5.0);
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                20.0,
                "the flag refused it"
            );
            game.hurt_player_by("minecraft:out_of_world", 100.0);
            assert!(
                game.player.as_ref().unwrap().health.health() <= 0.0,
                "out of world gets past the flag"
            );

            // A stand without saved data reads false, and the write lands in
            // the saved `Invulnerable` tag the item damage path refuses by.
            assert_eq!(game.entity_permanently_invulnerable(stand), Some(false));
            assert!(game.entity_set_invulnerable(stand, true));
            assert_eq!(game.entity_permanently_invulnerable(stand), Some(true));
            let raw = game.persist.raw.get(&14).cloned().unwrap();
            assert!(horizon_client::entity_nbt::flag(&raw, "Invulnerable"));
            // The timer tag rides along the same compound.
            assert!(game.entity_set_invulnerable_time(stand, 12));
            assert_eq!(game.entity_invulnerable_time(stand), Some(12));
            let raw = game.persist.raw.get(&14).cloned().unwrap();
            assert!(matches!(
                raw.get("invulnerable_time"),
                Some(horizon_core::nbt::Tag::Int(12))
            ));
        });
    }

    /// A lit stone floor under the origin, so a mob's tick survives it.
    fn floor(game: &mut super::Game<'_>) {
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
    }

    /// #916: `Entity.invulnerableTime` is its own field — never the living
    /// damage cooldown — on a mob and a player alike, and both survive a
    /// save and load.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_entity_timer_never_shares_the_living_cooldown() {
        use horizon_client::motion::PlayerMotion;
        super::super::native_entity_refs::tests::with_game(|game| {
            // The #170 recipe: two zombies on a lit stone floor, kept from
            // despawning while their tick runs.
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
            floor(game);
            let mob = game.entity(12).unwrap();
            let mob_ref = game.native_mob_ref(game.entity(12).unwrap()).unwrap();
            assert!(mob_ref.set_persistence_required(game));
            assert!(mob_ref.set_no_action_time(game, 650));

            // Different fields, counted down side by side by the base tick.
            assert!(game.entity_set_damage_cooldown_time(mob, 15));
            assert!(game.entity_set_invulnerable_time(mob, 5));
            game.tick_mobs().unwrap();
            assert_eq!(game.entity_damage_cooldown_time(mob), Some(14));
            assert_eq!(game.entity_invulnerable_time(mob), Some(4));
            game.mobs.clear();

            // The player's two, on the same clock.
            game.player = Some(PlayerMotion::new(game.motion, Vec3::ZERO).unwrap());
            game.lan_acting_uuid = Some(123);
            game.lan_acting_entity = Some(20);
            let player = game.entity(20).unwrap();
            assert!(game.entity_set_damage_cooldown_time(player, 7));
            assert!(game.entity_set_invulnerable_time(player, 3));
            game.player.as_mut().unwrap().health.advance_clock();
            assert_eq!(game.entity_damage_cooldown_time(player), Some(6));
            assert_eq!(game.entity_invulnerable_time(player), Some(2));

            // A mob's flag and timer survive a save and a load, as vanilla
            // writes them (`Invulnerable`, `invulnerable_time`).
            game.mobs.push(Mob::new(
                30,
                game.res
                    .mob_types()
                    .get("minecraft:zombie")
                    .unwrap()
                    .clone(),
                Vec3::new(0.5, 64.0, 0.5),
                0.0,
            ));
            game.mobs[0].invulnerable_flag = true;
            game.mobs[0].invulnerable_time = 9;
            let tag =
                horizon_client::entity_nbt::mob_to_nbt(&game.mobs[0], [0; 4], None, None, |_| None);
            let back =
                horizon_client::entity_nbt::mob_from_nbt(&tag, game.res.mob_types(), 30, None)
                    .unwrap();
            assert!(back.invulnerable_flag);
            assert_eq!(back.invulnerable_time, 9);
        });
    }

    /// #916, review: the window really refuses. A mod sets the timer and the
    /// entity no longer takes damage — nothing but what bypasses
    /// invulnerability — and enemies no longer see it as a target, until the
    /// window closes.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_window_refuses_damage_and_hides_the_mob_from_enemies() {
        use horizon_client::mobs::{Damage, DamageKind, Mob};
        use horizon_core::geometry::Vec3;
        super::super::native_entity_refs::tests::with_game(|game| {
            // The #170 recipe: two zombies on a lit stone floor, kept from
            // despawning while their tick runs.
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
            floor(game);
            let victim = game.entity(12).unwrap();
            let enemy = game.entity(13).unwrap();
            let keep = |game: &mut super::Game<'_>| {
                let m = game.native_mob_ref(game.entity(12).unwrap()).unwrap();
                m.set_persistence_required(game);
                m.set_no_action_time(game, 650);
            };
            keep(game);

            // The window refuses a hit nothing-bypassing; the void goes
            // through (`Entity.isInvulnerableToBase`).
            assert!(game.entity_set_invulnerable_time(victim, 60));
            let before = game.mobs[0].health;
            assert!(!game.mobs[0].damage(
                crate::game::entity::damage_level_of!(game),
                5.0,
                Damage::of(DamageKind::Generic),
            ));
            assert_eq!(game.mobs[0].health, before, "the window refused it");
            assert!(game.mobs[0].damage(
                crate::game::entity::damage_level_of!(game),
                100.0,
                Damage::of(DamageKind::OutOfWorld),
            ));
            assert!(game.mobs[0].health <= 0.0, "the void gets past");

            // `canBeSeenAsEnemy`: the facts enemies ask about carry the window
            // too (`Mob::seen`, `isInvulnerable`).
            game.mobs[0].health = 20.0;
            assert!(game.mobs[0].invulnerable_time > 0);
            assert!(game.mobs[0].is_invulnerable());
            assert!(game.mobs[0].seen(None).invulnerable);
            // ...and once the window has run out, the enemy sees it again.
            game.mobs[0].invulnerable_time = 1;
            game.tick_mobs().unwrap();
            assert_eq!(game.mobs[0].invulnerable_time, 0);
            assert!(!game.mobs[0].seen(None).invulnerable);
            let _ = enemy;
            keep(game);
        });
    }

    /// #916, review round 2: `Entity.setInvulnerable` on a player is the
    /// permanent flag, apart from `Abilities.invulnerable` — the abilities
    /// stay untouched, a survival hit is refused and a creative player's
    /// passes the flag (`isInvulnerableToBase`'s one exception), while the
    /// ability gate (a creative player's own mode) holds even a creative
    /// hit back, as vanilla `Player.hurtServer` does.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_players_permanent_flag_is_apart_from_their_abilities() {
        use horizon_client::motion::PlayerMotion;
        super::super::native_entity_refs::tests::with_game(|game| {
            game.player = Some(PlayerMotion::new(game.motion, Vec3::ZERO).unwrap());
            game.lan_acting_uuid = Some(123);
            game.lan_acting_entity = Some(20);
            let player = game.entity(20).unwrap();

            // The window refuses an ordinary hit; a creative player's
            // `player_attack` passes it (`DamageSource.isCreativePlayer`).
            assert!(game.entity_set_invulnerable_time(player, 60));
            game.mob_hits_player(5.0, [0.0; 3], "minecraft:player_attack");
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                20.0,
                "the window refused it"
            );
            game.mob_hits_player_from(5.0, [0.0; 3], "minecraft:player_attack", true);
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                15.0,
                "a creative player's hit passes the window"
            );
            game.player.as_mut().unwrap().health.invulnerable_time = 0;
            // The landed hit started the hurt cooldown: a later hit must
            // out-damage it to land at all (`LivingEntity.hurtServer`).
            game.player.as_mut().unwrap().health.hurt.cooldown = 0;
            game.player.as_mut().unwrap().health.hurt.last_hurt = 0.0;

            // The permanent flag is not the abilities: set_invulnerable
            // leaves `Abilities.invulnerable` alone.
            assert!(game.entity_set_invulnerable(player, true));
            assert!(
                !game.player.as_ref().unwrap().invulnerable,
                "the abilities are untouched"
            );
            game.mob_hits_player(5.0, [0.0; 3], "minecraft:player_attack");
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                15.0,
                "the flag refused the survival hit"
            );
            game.mob_hits_player_from(5.0, [0.0; 3], "minecraft:player_attack", true);
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                10.0,
                "and a creative player's hit passes the flag"
            );
            game.player.as_mut().unwrap().health.hurt.cooldown = 0;
            game.player.as_mut().unwrap().health.hurt.last_hurt = 0.0;

            // The ability flag is another gate (`Player.hurtServer`): even a
            // creative player's hit does not pass it, as in vanilla.
            game.player.as_mut().unwrap().invulnerable = true;
            game.mob_hits_player_from(5.0, [0.0; 3], "minecraft:player_attack", true);
            assert_eq!(
                game.player.as_ref().unwrap().health.health(),
                10.0,
                "the ability flag holds even a creative hit back"
            );
        });
    }

    /// #916, review: a non-mob's saved window counts down each tick, and the
    /// item damage path refuses by it.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_non_mobs_saved_window_counts_down_and_refuses() {
        use horizon_client::entities::ArmorStand;
        use horizon_core::geometry::Vec3;
        super::super::native_entity_refs::tests::with_game(|game| {
            game.armor_stands
                .push(ArmorStand::new(14, Vec3::new(0.5, 64.0, 0.5), 0.0));
            let stand = game.entity(14).unwrap();
            assert!(game.entity_set_invulnerable_time(stand, 20));
            // Entity.baseTick, over the saved tag.
            game.tick_saved_invulnerable_time(14);
            game.tick_saved_invulnerable_time(14);
            assert_eq!(game.entity_invulnerable_time(stand), Some(18));
            // Down to zero it stays, and reads as no window.
            for _ in 0..18 {
                game.tick_saved_invulnerable_time(14);
            }
            assert_eq!(game.entity_invulnerable_time(stand), Some(0));
        });
    }

    /// #916, review round 2: the dragon's saved flag refuses `/damage`
    /// except from a creative player — the one refusal every saved kind
    /// shares (`saved_refuses_damage`).
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_dragons_saved_flag_refuses_all_but_a_creative_player() {
        use super::super::dragon::Dragon;
        use horizon_client::mobs::DamageKind;
        super::super::native_entity_refs::tests::with_game(|game| {
            let mut dragon = Dragon::new(Vec3::ZERO, 0.0, 0);
            dragon.id = 15;
            game.dragon = Some(dragon);
            let entity = game.entity(15).unwrap();
            assert!(game.entity_set_invulnerable(entity, true));

            // A survival player's blow: refused.
            game.creative = false;
            assert!(!game.command_damage_dragon(
                15,
                10.0,
                "minecraft:generic",
                DamageKind::Generic,
                (Some(super::Target::Player), None, None),
            ));
            // A creative player's blow passes the flag (`isCreativePlayer`).
            game.creative = true;
            assert!(game.command_damage_dragon(
                15,
                10.0,
                "minecraft:generic",
                DamageKind::Generic,
                (Some(super::Target::Player), None, None),
            ));
            // And the void needs no creative help (`#bypasses_invulnerability`).
            game.creative = false;
            assert!(game.command_damage_dragon(
                15,
                1000.0,
                "minecraft:out_of_world",
                DamageKind::OutOfWorld,
                (Some(super::Target::Player), None, None),
            ));
        });
    }
}
