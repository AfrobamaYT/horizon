//! State flags of a living entity for native server mods, through a
//! `LivingRef` (docs/modding/src/how-to/native-mods.md, "Entity and level
//! handles"): the glide (`LivingEntity.isFallFlying` and `stopFallFlying`),
//! the sleep of a player or a villager (`isSleeping`, `getSleepingPos`,
//! `startSleeping`, `stopSleeping`, `setSleepingPos` and `clearSleepingPos`),
//! the jump of the player in place (`isJumping`, `setJumping`), climbing
//! (`LivingEntity.onClimbable`), the baby age (`LivingEntity.isBaby` and
//! `Mob.setBaby`) and the mob flags `Mob.isNoAi`/`setNoAi`,
//! `isPersistenceRequired`/`setPersistenceRequired` and
//! `requiresCustomPersistence`. Reads return `None` and writes `false` for a
//! stale handle, and for a kind Horizon keeps no such state for (each doc
//! says which).
use super::{Game, commands::Target};
use horizon_client::mobs::villager;
use horizon_core::geometry::Vec3;
use horizon_core::position::BlockPos;
use horizon_world::climbing::{self, ClimbingWorld};
use horizon_world::registry::BlockStateId;
use mod_api::LivingRef;

impl Game<'_> {
    /// Whether the handle names the local player in place: the player the
    /// game itself runs, not a guest's swapped-in turn (whose client holds
    /// its own sleep). On a dedicated server every turn is a guest's.
    fn native_living_local_player(&self, living: LivingRef) -> bool {
        self.native_living_target(living) == Some(Target::Player) && self.lan_guest_swaps.is_empty()
    }

    /// The ticks the entity has glided (`LivingEntity.getFallFlyingTicks`),
    /// counted from 0 again the tick after the glide ends. The player in
    /// place and a mob keep the count themselves. None for an armor stand
    /// or the dragon.
    pub(super) fn native_living_gliding_ticks(&self, living: LivingRef) -> Option<i32> {
        match self.native_living_read_target(living)? {
            Target::Player => Some(self.player.as_ref()?.fall_fly_ticks),
            Target::Guest(_) => Some(0),
            Target::Mob(id) => Some(
                self.mobs
                    .iter()
                    .find(|m| m.id == id && !m.removed)?
                    .extra
                    .fall_fly_ticks,
            ),
            _ => None,
        }
    }
    /// Whether the entity glides on elytra (`LivingEntity.isFallFlying`;
    /// Paper's `LivingEntity#isGliding`). The player in place and the guests
    /// answer from their fall-flying flag, a mob from its own. None for an
    /// armor stand or the dragon: the game keeps no glide for them.
    pub(super) fn native_living_is_gliding(&self, living: LivingRef) -> Option<bool> {
        match self.native_living_read_target(living)? {
            Target::Player => Some(self.player.as_ref()?.fall_flying),
            Target::Guest(conn) => Some(self.command_guest(conn)?.fall_flying),
            Target::Mob(id) => Some(
                self.mobs
                    .iter()
                    .find(|m| m.id == id && !m.removed)?
                    .extra
                    .fall_flying,
            ),
            _ => None,
        }
    }
    /// `LivingEntity.stopFallFlying`: clears the glide flag, so the glide's
    /// wear and sound stop and the next fall is a fall. The players around
    /// learn it with the entity's next flag sync, as they do when a glide
    /// ends on its own. True for a player, a guest or a mob; false for an
    /// armor stand or the dragon.
    pub(super) fn native_living_stop_gliding(&mut self, living: LivingRef) -> bool {
        match self.native_living_target(living) {
            Some(Target::Player) => {
                let Some(p) = self.player.as_mut() else {
                    return false;
                };
                p.fall_flying = false;
                true
            }
            Some(Target::Guest(conn)) => {
                let Some(g) = self
                    .lan
                    .as_mut()
                    .and_then(|h| h.hub.guests.iter_mut().find(|g| g.conn == conn))
                else {
                    return false;
                };
                g.fall_flying = false;
                true
            }
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter_mut().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                m.extra.fall_flying = false;
                true
            }
            _ => false,
        }
    }

    /// Whether the entity sleeps in a bed (`LivingEntity.isSleeping`): the
    /// player in place, a guest, or a villager or wandering trader asleep in
    /// its bed. Other mobs never sleep. None for an armor stand or the
    /// dragon.
    pub(super) fn native_living_is_sleeping(&self, living: LivingRef) -> Option<bool> {
        match self.native_living_read_target(living)? {
            Target::Player => Some(self.sleeping.is_some()),
            Target::Guest(conn) => Some(self.command_guest(conn)?.sleeping),
            Target::Mob(id) => Some(
                self.mobs
                    .iter()
                    .find(|m| m.id == id && !m.removed)
                    .map(villager::sleeping)
                    .unwrap_or(false),
            ),
            _ => None,
        }
    }
    /// The head of the bed the entity sleeps in (`LivingEntity.getSleepingPos`),
    /// as the block position it lies at. A guest keeps the bed it last slept
    /// in while it sleeps, a villager or wandering trader its own. None for
    /// an awake entity, another mob, an armor stand or the dragon.
    pub(super) fn native_living_sleeping_pos(&self, living: LivingRef) -> Option<[i32; 3]> {
        match self.native_living_read_target(living)? {
            Target::Player => self.sleeping,
            Target::Guest(conn) => {
                let g = self.command_guest(conn)?;
                g.bed.filter(|_| g.sleeping)
            }
            Target::Mob(id) => {
                villager::sleeping_pos(self.mobs.iter().find(|m| m.id == id && !m.removed)?)
            }
            _ => None,
        }
    }
    /// `LivingEntity.startSleeping`: the entity lies in the bed whose head
    /// `pos` names, with the effect the game's own sleep has. The player in
    /// place is moved onto the bed, put to sleep and occupies it; a
    /// villager or wandering trader takes the bed the game's evening gives
    /// it (an occupied bed leaves it standing). False for a guest, whose
    /// client tucks itself in, another mob, an armor stand or the dragon.
    pub(super) fn native_living_start_sleeping(
        &mut self,
        living: LivingRef,
        pos: [i32; 3],
    ) -> bool {
        match self.native_living_target(living) {
            Some(Target::Player) if self.native_living_local_player(living) => self.lie_in_bed(pos),
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                if !villager::sleeping(m) && m.extra.villager.is_none() {
                    // Only a villager or a wandering trader has a bed to take.
                    return false;
                }
                let mut mobs = std::mem::take(&mut self.mobs);
                let w = self.mob_world_base(&[]);
                let mut events = Vec::new();
                let slept = mobs
                    .iter_mut()
                    .find(|m| m.id == id)
                    .is_some_and(|m| villager::start_sleeping(m, pos, &w, &mut events));
                self.mobs = mobs;
                let _ = self.handle_mob_events(events);
                slept
            }
            _ => false,
        }
    }
    /// `LivingEntity.stopSleeping`: the sleeper stands up beside the bed, as
    /// the game's own waking does, with its pose and packets. True when the
    /// entity slept; false for a guest, whose client stands itself up,
    /// another mob, an armor stand or the dragon.
    pub(super) fn native_living_stop_sleeping(&mut self, living: LivingRef) -> bool {
        match self.native_living_target(living) {
            Some(Target::Player) if self.native_living_local_player(living) => {
                let slept = self.sleeping.is_some();
                if slept {
                    self.wake_up();
                }
                slept
            }
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                if !villager::sleeping(m) {
                    return false;
                }
                let mut mobs = std::mem::take(&mut self.mobs);
                let w = self.mob_world_base(&[]);
                let mut events = Vec::new();
                if let Some(m) = mobs.iter_mut().find(|m| m.id == id) {
                    villager::stop_sleeping(m, &w, &mut events);
                }
                self.mobs = mobs;
                let _ = self.handle_mob_events(events);
                true
            }
            _ => false,
        }
    }
    /// `LivingEntity.setSleepingPos`: the bed `pos` names becomes the one
    /// the player sleeps in, without moving them; the next entity sync
    /// tells the players around they sleep there, as it tells them the
    /// game's own sleep. The villagers' beds belong to the evening's bed
    /// path (`start_sleeping`). False for a guest, a mob, an armor stand or
    /// the dragon.
    pub(super) fn native_living_set_sleeping_pos(
        &mut self,
        living: LivingRef,
        pos: [i32; 3],
    ) -> bool {
        if self.native_living_local_player(living) {
            self.sleeping = Some(pos);
            return true;
        }
        false
    }
    /// `LivingEntity.clearSleepingPos`: the player no longer names a bed, so
    /// `is_sleeping` is false and the next entity sync stands them up.
    /// False for a guest, a mob, an armor stand or the dragon.
    pub(super) fn native_living_clear_sleeping_pos(&mut self, living: LivingRef) -> bool {
        if self.native_living_local_player(living) {
            self.sleeping = None;
            return true;
        }
        false
    }
    /// Whether the entity jumps (`LivingEntity.isJumping`; Paper's
    /// `LivingEntity#isJumping`): the player in place answers from their
    /// own jump state. The guests' jump is their client's, and a mob has no
    /// jump controller in the game's own AI, so both read false. None for
    /// an armor stand or the dragon.
    pub(super) fn native_living_is_jumping(&self, living: LivingRef) -> Option<bool> {
        match self.native_living_read_target(living)? {
            Target::Player => Some(self.player.as_ref()?.jumping),
            Target::Guest(_) => Some(false),
            Target::Mob(_) => Some(false),
            _ => None,
        }
    }
    /// `LivingEntity.setJumping`: whether the player in place's next step
    /// jumps, as their own key does. False for a guest outside its turn,
    /// whose client holds its own jump, a mob, an armor stand or the
    /// dragon.
    pub(super) fn native_living_set_is_jumping(
        &mut self,
        living: LivingRef,
        jumping: bool,
    ) -> bool {
        match self.native_living_target(living) {
            Some(Target::Player) => {
                let Some(p) = self.player.as_mut() else {
                    return false;
                };
                p.jumping = jumping;
                true
            }
            _ => false,
        }
    }
    /// Whether the entity is on a climbable block this tick
    /// (`LivingEntity.onClimbable`; Paper's `LivingEntity#isClimbing`), by
    /// the block at its feet: a ladder, a vine, a scaffold or an open
    /// trapdoor above a ladder. A spectator never climbs. None for an armor
    /// stand or the dragon.
    pub(super) fn native_living_is_climbing(&self, living: LivingRef) -> Option<bool> {
        let target = self.native_living_read_target(living)?;
        let (position, spectator) = match target {
            Target::Player => {
                let p = self.player.as_ref()?;
                (p.position, p.spectator)
            }
            Target::Guest(conn) => {
                let g = self.command_guest(conn)?;
                (
                    Vec3::new(g.position[0], g.position[1], g.position[2]),
                    g.spectator(),
                )
            }
            Target::Mob(id) => (
                self.mobs
                    .iter()
                    .find(|m| m.id == id && !m.removed)?
                    .position,
                false,
            ),
            _ => return None,
        };
        let mut world = FlagClimbing {
            world: &self.world,
            position: BlockPos::containing(position),
            spectator,
        };
        Some(climbing::on_climbable(&mut world, &self.motion.climbing).unwrap_or(false))
    }
    /// Whether the entity is a baby (`LivingEntity.isBaby`): a mob answers
    /// from its age, negative until it grows up. A player, an armor stand
    /// and the dragon are never babies.
    pub(super) fn native_living_baby(&self, living: LivingRef) -> Option<bool> {
        match self.native_living_read_target(living)? {
            Target::Mob(id) => Some(
                self.mobs
                    .iter()
                    .find(|m| m.id == id && !m.removed)?
                    .extra
                    .age
                    < 0,
            ),
            Target::Player | Target::Guest(_) | Target::Stand(_) | Target::Dragon(_) => Some(false),
            _ => None,
        }
    }
}

/// `LivingEntity.onClimbable` reads the world through `ClimbingWorld`; this
/// walks the level the flag's entity stands in, at its feet.
struct FlagClimbing<'a> {
    world: &'a horizon_client::world::World,
    position: BlockPos,
    spectator: bool,
}

impl ClimbingWorld for FlagClimbing<'_> {
    type Error = std::convert::Infallible;
    fn is_spectator(&mut self) -> Result<bool, Self::Error> {
        Ok(self.spectator)
    }
    fn is_fall_flying(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn block_position(&mut self) -> Result<BlockPos, Self::Error> {
        Ok(self.position)
    }
    fn in_block_state(&mut self) -> Result<BlockStateId, Self::Error> {
        self.block_state(self.position)
    }
    fn block_state(&mut self, p: BlockPos) -> Result<BlockStateId, Self::Error> {
        Ok(self.world.block(p.x, p.y, p.z))
    }
    fn set_last_climbable_position(&mut self, _: BlockPos) {}
}

#[cfg(test)]
mod tests {
    use super::super::Game;
    use super::super::native_entity_refs::tests::with_game;
    use horizon_client::mobs::{Mob, villager};
    use horizon_core::geometry::Vec3;
    use mod_api::{EntityRef, LivingRef, NativeWorldEpoch};

    /// A live cow `id` at `(5.5, 64, 5.5)`, with its handle.
    fn cow(game: &mut Game<'_>, id: u32) -> LivingRef {
        let kind = game.res.mob_types().get("minecraft:cow").unwrap().clone();
        game.mobs
            .push(Mob::new(id, kind, Vec3::new(5.5, 64.0, 5.5), 0.0));
        game.native_living(game.native_entity(id).unwrap()).unwrap()
    }

    /// The player in place, standing at the block `[0, 64, 0]`'s feet.
    fn standing_player(game: &mut Game<'_>) -> LivingRef {
        use horizon_client::motion::PlayerMotion;
        game.options.dedicated = Some(Vec::new());
        game.player = Some(PlayerMotion::new(game.motion, Vec3::ZERO).unwrap());
        game.lan_acting_uuid = Some(123);
        game.lan_acting_entity = Some(20);
        game.native_living(game.native_entity(20).unwrap()).unwrap()
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn mob_flags_write_the_stored_state_and_survive_the_save() {
        with_game(|game| {
            let mob = cow(game, 12);
            let game = &mut *game;
            // The mob flags: off, on, and what each write leaves behind.
            // They are MobRef's own methods (Mob.isNoAi and its kin).
            let mob_ref = game
                .native_mob_ref(game.native_entity(12).unwrap())
                .unwrap();
            assert_eq!(game.native_mob_no_ai(mob_ref), Some(false));
            assert!(game.native_mob_set_no_ai(mob_ref, true));
            assert!(game.mobs[0].no_ai);
            assert_eq!(game.native_mob_no_ai(mob_ref), Some(true));
            assert_eq!(game.native_mob_persistence_required(mob_ref), Some(false));
            assert_eq!(
                game.native_mob_requires_custom_persistence(mob_ref),
                Some(false)
            );
            assert!(game.native_mob_set_persistence_required(mob_ref));
            assert!(game.mobs[0].persistent);
            assert_eq!(game.native_mob_persistence_required(mob_ref), Some(true));
            // The baby age: a full day of growing up, and grown on the spot.
            assert_eq!(game.native_living_baby(mob), Some(false));
            assert!(game.native_mob_set_baby(mob_ref, true));
            assert_eq!(game.mobs[0].extra.age, -24000);
            assert_eq!(game.native_living_baby(mob), Some(true));
            let _ = &mob_ref;
            // The glide flag clears the way the game's own glide end does.
            game.mobs[0].extra.fall_flying = true;
            game.mobs[0].extra.fall_fly_ticks = 9;
            assert_eq!(game.native_living_is_gliding(mob), Some(true));
            assert_eq!(game.native_living_gliding_ticks(mob), Some(9));
            assert!(game.native_living_stop_gliding(mob));
            assert!(!game.mobs[0].extra.fall_flying);
            // A villager's bed helpers answer for an awake cow.
            assert!(!villager::sleeping(&game.mobs[0]));
            assert_eq!(game.native_living_is_sleeping(mob), Some(false));
            assert_eq!(game.native_living_sleeping_pos(mob), None);
            assert!(!game.native_living_start_sleeping(mob, [5, 64, 5]));
            assert!(!game.native_living_stop_sleeping(mob));
            // NoAI, PersistenceRequired and Age are the save's own tags.
            let tag = horizon_client::entity_nbt::mob_to_nbt(
                &game.mobs[0],
                [1, 2, 3, 4],
                None,
                Some(game.pack.assets.registry),
            );
            let restored = horizon_client::entity_nbt::mob_from_nbt(
                &tag,
                game.res.mob_types(),
                7,
                Some(game.pack.assets.registry),
            )
            .unwrap();
            assert!(restored.no_ai && restored.persistent);
            assert_eq!(restored.extra.age, -24000);
            // A zombie keeps its flags through the same cycle (its age
            // never saves, as vanilla's zombie baby never grows).
            let kind = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs
                .push(Mob::new(13, kind, Vec3::new(5.5, 64.0, 5.5), 0.0));
            let zombie_ref = game
                .native_mob_ref(game.native_entity(13).unwrap())
                .unwrap();
            assert!(game.native_mob_set_persistence_required(zombie_ref));
            assert!(game.native_mob_set_no_ai(zombie_ref, true));
            let tag = horizon_client::entity_nbt::mob_to_nbt(
                &game.mobs[1],
                [5, 6, 7, 8],
                None,
                Some(game.pack.assets.registry),
            );
            let restored = horizon_client::entity_nbt::mob_from_nbt(
                &tag,
                game.res.mob_types(),
                8,
                Some(game.pack.assets.registry),
            )
            .unwrap();
            assert!(restored.persistent && restored.no_ai);
            // A grown-up cow reads back as one, too.
            assert!(game.native_mob_set_baby(mob_ref, false));
            assert_eq!(game.mobs[0].extra.age, 0);
            assert_eq!(game.native_living_baby(mob), Some(false));
            // A stale handle reads None and writes false.
            let stale = LivingRef::new(EntityRef::new(
                NativeWorldEpoch::new(mob.entity().epoch().get() + 1),
                12,
            ));
            assert!(game.native_mob_ref(stale.entity()).is_none());
            assert_eq!(game.native_living_is_sleeping(stale), None);
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn player_flags_read_their_own_body_and_the_bed_takes_them() {
        use horizon_world::chunk::BlockState;
        with_game(|game| {
            let player = standing_player(game);
            let game = &mut *game;
            game.world.chunks.insert(
                (0, 0),
                std::sync::Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let registry = game.pack.assets.registry;
            let ladder = registry.default_state("minecraft:ladder").unwrap();
            // No climbable block under the player yet.
            assert_eq!(game.native_living_is_climbing(player), Some(false));
            game.world.set(0, 0, 0, ladder).unwrap();
            assert_eq!(game.native_living_is_climbing(player), Some(true));
            // The glide and jump flags live on the player's own body.
            assert_eq!(game.native_living_gliding_ticks(player), Some(0));
            game.player.as_mut().unwrap().fall_fly_ticks = 4;
            assert_eq!(game.native_living_gliding_ticks(player), Some(4));
            game.player.as_mut().unwrap().fall_fly_ticks = 0;
            assert_eq!(game.native_living_is_gliding(player), Some(false));
            game.player.as_mut().unwrap().fall_flying = true;
            assert_eq!(game.native_living_is_gliding(player), Some(true));
            assert!(game.native_living_stop_gliding(player));
            assert!(!game.player.as_ref().unwrap().fall_flying);
            assert_eq!(game.native_living_is_jumping(player), Some(false));
            assert!(game.native_living_set_is_jumping(player, true));
            assert!(game.player.as_ref().unwrap().jumping);
            // No bed named, no sleep: the straw bed's head has no shape.
            assert_eq!(game.native_living_is_sleeping(player), Some(false));
            assert_eq!(game.native_living_sleeping_pos(player), None);
            assert!(!game.native_living_start_sleeping(player, [5, 64, 5]));
            // A bed's head takes the player, and waking stands them up.
            let head = registry
                .resolve(&BlockState {
                    name: "minecraft:red_bed".into(),
                    properties: [("facing", "north"), ("occupied", "false"), ("part", "head")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                })
                .unwrap();
            game.world.set(5, 64, 5, head).unwrap();
            assert!(game.native_living_start_sleeping(player, [5, 64, 5]));
            assert_eq!(game.native_living_is_sleeping(player), Some(true));
            assert_eq!(game.native_living_sleeping_pos(player), Some([5, 64, 5]));
            assert!(game.native_living_stop_sleeping(player));
            assert_eq!(game.native_living_is_sleeping(player), Some(false));
            assert_eq!(game.native_living_sleeping_pos(player), None);
            // Setting and clearing the bed's name moves nothing.
            assert!(game.native_living_set_sleeping_pos(player, [6, 64, 6]));
            assert_eq!(game.native_living_is_sleeping(player), Some(true));
            assert_eq!(game.native_living_sleeping_pos(player), Some([6, 64, 6]));
            assert!(game.native_living_clear_sleeping_pos(player));
            assert_eq!(game.native_living_is_sleeping(player), Some(false));
            assert_eq!(game.native_living_sleeping_pos(player), None);
            // A player is never a baby, and no mob handle names them, so
            // every MobRef flag write refuses.
            assert_eq!(game.native_living_baby(player), Some(false));
            let player_mob = game.native_mob_ref(player.entity());
            assert!(player_mob.is_none(), "a player is no mob");
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_summons_join_write_survives_the_spawn() {
        use mod_api::{EntityJoinLevelEvent, Flow, Hooks, Host, ModEntry, ServerHost};
        struct Joiner;
        #[mod_api::__subscriptions]
        impl Hooks for Joiner {
            fn entity_join_level(
                &self,
                host: &mut dyn ServerHost,
                event: &mut EntityJoinLevelEvent<'_>,
            ) -> Flow {
                if event.kind == "minecraft:zombie"
                    && let Some(er) = Host::entity(&*host, event.entity)
                    && let Some(m) = Host::mob(&*host, er)
                {
                    let _ = m.set_persistence_required(&mut *host);
                    let _ = m.set_no_ai(&mut *host, true);
                    let _ = m.set_baby(&mut *host, true);
                }
                Flow::Continue
            }
        }
        mod_api::testing::with_mods(
            vec![ModEntry::new("join_persist", Box::new(Joiner))].into(),
            || {
                with_game(|game| {
                    let before = game.mobs.len();
                    game.spawn_mob_spawned(
                        "zombie",
                        [5.0, 64.0, 5.0],
                        super::super::creatures::SpawnSite::Finalizing(
                            horizon_client::mobs::spawn_rules::Reason::Command,
                        ),
                    )
                    .unwrap();
                    let m = &game.mobs[before];
                    assert!(m.persistent, "the join hook's write survives");
                    assert!(m.no_ai && m.extra.age == -24000);
                });
            },
        );
        // Without a mod the summoned mob stays plain vanilla: not
        // persistent, as SummonCommand never calls setPersistenceRequired.
        with_game(|game| {
            let before = game.mobs.len();
            game.spawn_mob_spawned(
                "zombie",
                [5.0, 64.0, 5.0],
                super::super::creatures::SpawnSite::Finalizing(
                    horizon_client::mobs::spawn_rules::Reason::Command,
                ),
            )
            .unwrap();
            assert!(!game.mobs[before].persistent, "a summon stays vanilla");
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_guests_flags_read_from_its_own_record() {
        use super::super::lan::LanSettings;
        use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
        use horizon_protocol::packets::configuration::ClientInformation;
        use std::sync::{
            Arc,
            atomic::{AtomicI32, AtomicUsize},
        };
        with_game(|game| {
            let settings = LanSettings {
                port: 0,
                guest_commands: false,
                force_game_mode: false,
            };
            let net = NetConfig {
                listen: false,
                announce: false,
                lod_distance: 0,
                ..NetConfig::lan(0, String::new(), 2)
            };
            let config = HubConfig {
                default_game_mode: 0,
                guest_commands: false,
                force_game_mode: false,
                hardcore: false,
                view_distance: 2,
                simulation_distance: 2,
                dimension: game.dim.id().to_owned(),
                allow_flight: true,
                seed: 0,
                max_players: 2,
                integrated: true,
                levels: game.level_shapes,
            };
            game.lan_host(settings, net, config).unwrap();
            let (tx, _rx) = std::sync::mpsc::channel();
            let mut guest = Guest::new(
                Joined {
                    conn: 12,
                    uuid: 12,
                    name: "guest12".into(),
                    properties: Vec::new(),
                    info: ClientInformation::new(8),
                    brand: String::new(),
                    sender: PlayerSender::new(tx, Arc::new(AtomicUsize::new(0))),
                    latency: Arc::new(AtomicI32::new(0)),
                    address: "127.0.0.1:1".parse().unwrap(),
                    expected: false,
                },
                12,
                [3.0, 64.0, 5.0],
                10,
            );
            guest.placed = true;
            guest.fall_flying = true;
            guest.sleeping = true;
            guest.bed = Some([9, 64, 9]);
            game.lan.as_mut().unwrap().hub.guests.push(guest);
            let guest = game.native_living(game.native_entity(12).unwrap()).unwrap();
            let game = &mut *game;
            assert_eq!(game.native_living_is_gliding(guest), Some(true));
            assert_eq!(game.native_living_gliding_ticks(guest), Some(0));
            assert_eq!(game.native_living_is_sleeping(guest), Some(true));
            assert_eq!(game.native_living_sleeping_pos(guest), Some([9, 64, 9]));
            // The glide flag writes to the guest's own swapped-in body.
            assert!(game.native_living_stop_gliding(guest));
            assert!(!game.lan.as_ref().unwrap().hub.guests[0].fall_flying);
            // The guest's sleep is its client's; the server only reads it.
            assert!(!game.native_living_start_sleeping(guest, [9, 64, 9]));
            assert!(!game.native_living_stop_sleeping(guest));
            assert!(!game.native_living_set_sleeping_pos(guest, [9, 64, 9]));
            assert!(!game.native_living_clear_sleeping_pos(guest));
            assert!(game.lan.as_ref().unwrap().hub.guests[0].sleeping);
            assert_eq!(game.native_living_is_jumping(guest), Some(false));
            assert!(!game.native_living_set_is_jumping(guest, true));
            assert_eq!(game.native_living_baby(guest), Some(false));
            assert!(game.native_mob_ref(guest.entity()).is_none());
        });
    }
}
