//! Entity and level handles for native server mods (docs/native-mods.md,
//! "Entity and level handles", #7387): `native_entity(id)` turns an event's
//! entity id into an `EntityRef`, and each `native_entity_*` helper reads the
//! entity a handle names. They read where the world is simulated: on a client,
//! with no world open, or for a handle of an earlier open, they return `None`.
//! The entities are those of the level in place (the one a hook runs in) and
//! the server's players, whatever their level.
use super::{
    Dim, Game,
    commands::Target,
    entity::{Entity as _, entity_lists_of, for_each_entity},
};
use horizon_client::mod_api::{
    EntityRef, LevelRef, NativeBlockFace, NativeBlockShapeKind, NativeBlockUpdateFlags,
    NativeClipBlock, NativeClipFluid, NativeClipHit, NativeSupportType,
};
use horizon_core::geometry::{Aabb, Vec3};
use horizon_world::registry::BlockStateId;

impl Game<'_> {
    /// What a handle names now, or None for a stale or foreign one.
    pub(super) fn native_entity_target(&self, entity: EntityRef) -> Option<Target> {
        if self.remote.is_some() || self.native_lifecycle.epoch() != Some(entity.epoch()) {
            return None;
        }
        let id = i32::try_from(entity.id()).ok()?;
        if id == 0 {
            return None;
        }
        let player = self
            .lan
            .iter()
            .flat_map(|h| h.hub.guests.iter())
            .find(|g| g.placed && g.entity_id == id);
        if let Some(g) = player {
            // `as_guest` swaps the acting player's state into the game.
            let acting = self.lan_acting == Some(g.conn) && self.player.is_some();
            return Some(if acting {
                Target::Player
            } else {
                Target::Guest(g.conn)
            });
        }
        match self.level_entity(id)? {
            Target::Mob(id) if !self.mobs.iter().any(|m| m.id == id && !m.removed) => None,
            t => Some(t),
        }
    }
    /// The entity `id` (the id events carry) as a handle for the `entity_*`
    /// methods: None on a client, with no world open, or for no such entity
    /// in the level in place or among the server's players. API
    /// @API:entity_handles@.
    pub(super) fn native_entity(&self, id: u32) -> Option<EntityRef> {
        let entity = EntityRef::new(self.native_lifecycle.epoch()?, id);
        self.native_entity_target(entity).map(|_| entity)
    }
    /// The level `dimension` (`minecraft:the_nether`) as a handle: None on a
    /// client, with no world open, or for a dimension the server hasn't
    /// loaded. API @API:entity_handles@.
    pub(super) fn native_level(&self, dimension: &str) -> Option<LevelRef> {
        if self.remote.is_some() {
            return None;
        }
        let epoch = self.native_lifecycle.epoch()?;
        let dim = std::iter::once(self.dim)
            .chain(self.levels.parked_dims())
            .find(|d| d.id() == dimension)?;
        Some(LevelRef::new(epoch, dim.id()))
    }
    /// Entities whose bounding boxes intersect this box (`Level.getEntities`).
    /// Face-only contact does not intersect; spectator players are excluded.
    /// Returns None on a client, for a stale handle, or for a parked level:
    /// its entity handles cannot resolve until the level is in place. The walk
    /// is in entity-list order followed by players; each overlap requires a
    /// handle lookup, so cost grows with the box's contents. Tridents cannot
    /// yet resolve; frames, paintings, other special entities and the dragon
    /// are not on the entity-list walk.
    /// During a player's action with the hub taken out, players including the
    /// acting player are not listed: their handles cannot resolve there yet.
    /// During `hears_projectile_impacts`, the projectile list is taken out of
    /// the game and is not visible; it is restored for `projectile_impact`.
    /// API @API:level_entities_in@.
    pub(super) fn native_level_entities_in(
        &self,
        level: LevelRef,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<Vec<EntityRef>> {
        if self.remote.is_some()
            || self.native_lifecycle.epoch() != Some(level.epoch())
            || level.dimension() != self.dim.id()
        {
            return None;
        }
        let bounds = Aabb::new(
            Vec3::new(min[0], min[1], min[2]),
            Vec3::new(max[0], max[1], max[2]),
        );
        let mut ids = Vec::new();
        for_each_entity!(entity_lists_of!(self), entity => {
            // The join flush assigns zero IDs later; a handle could alias another entity.
            if entity.id() != 0 && !entity.is_removed() && bounds.intersects(entity.bounds()) {
                ids.push(entity.id());
            }
        });
        for guest in self.lan.iter().flat_map(|h| &h.hub.guests) {
            if !guest.placed || guest.dimension != level.dimension() {
                continue;
            }
            let body = if self.lan_acting == Some(guest.conn)
                && let Some(player) = &self.player
            {
                if self.mode_id() == 3 {
                    continue;
                }
                player.bounds()
            } else {
                if guest.game_mode == 3 {
                    continue;
                }
                guest.bounds()
            };
            if bounds.intersects(body)
                && let Ok(id) = u32::try_from(guest.entity_id)
            {
                ids.push(id);
            }
        }
        Some(
            ids.into_iter()
                .map(|id| EntityRef::new(level.epoch(), id))
                .filter(|handle| self.native_entity_target(*handle).is_some())
                .collect(),
        )
    }
    /// Set a block in the level in place (`Level.setBlock`). Parked levels,
    /// stale handles, clients and unloaded or out-of-height positions return
    /// false. Client-update and client-render flags are not supported; Horizon
    /// sends every change to clients. API @API:level_block_writes@.
    pub(super) fn native_level_set_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        state: BlockStateId,
        flags: NativeBlockUpdateFlags,
    ) -> bool {
        if !self.native_level_write_allowed(level, pos) {
            return false;
        }
        self.native_set_block(level.dimension(), pos, state, flags.bits())
    }
    /// Destroy a block in the level in place (`Level.destroyBlock`), optionally
    /// dropping its resources. Parked levels and invalid positions return false.
    /// API @API:level_block_writes@.
    pub(super) fn native_level_destroy_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        drop: bool,
    ) -> bool {
        if !self.native_level_write_allowed(level, pos)
            || (self.serving.is_none() && self.options.dedicated.is_none())
            || self.dim.id() != level.dimension()
        {
            return false;
        }
        let at = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
        let changed = self.with_level(|b, level, ticks| b.destroy_block(level, ticks, at, drop));
        let _ = self.process_level_events();
        changed
    }
    fn native_level_write_allowed(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        self.remote.is_none()
            && self.native_lifecycle.epoch() == Some(level.epoch())
            && self.world.loaded(pos[0], pos[2])
            && (self.world.layout().min_y()..self.world.layout().max_y()).contains(&pos[1])
    }
    /// Block-local outline or collision boxes with an empty context. Interaction,
    /// visual, occlusion and support shapes are not exposed. Returns None on a
    /// client, for a stale handle, or for a closed level. Unloaded blocks read
    /// as air; collision errors return no boxes, like `native_block_collision`.
    /// API @API:level_shapes@.
    pub(super) fn native_level_block_shape(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        kind: NativeBlockShapeKind,
    ) -> Option<Vec<[f64; 6]>> {
        if self.remote.is_some() || self.native_lifecycle.epoch() != Some(level.epoch()) {
            return None;
        }
        let (world, anims) = if self.dim.id() == level.dimension() {
            (&self.world, &self.be_anims)
        } else {
            let dim = self
                .levels
                .parked_dims()
                .into_iter()
                .find(|dim| dim.id() == level.dimension())?;
            (
                self.levels.parked_world(dim)?,
                self.levels.parked_anims(dim)?,
            )
        };
        Some(match kind {
            NativeBlockShapeKind::Outline => self
                .outline_boxes_in_context(world, anims, pos, false)
                .into_iter()
                .map(|b| std::array::from_fn(|i| b[i] - f64::from(pos[i % 3])))
                .collect(),
            NativeBlockShapeKind::Collision => self.native_block_collision_in(world, pos),
        })
    }

    /// Gameplay's cached face support in the current or a parked level. Returns
    /// None for clients, stale or closed levels, unloaded or out-of-height blocks,
    /// dynamic-shape states (including moving pistons and shulker boxes), or an
    /// exporter error. Dynamic support is a follow-up on #29. False means not
    /// sturdy. API @API:level_shapes@.
    pub(super) fn native_level_block_face_sturdy(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        face: NativeBlockFace,
        support: NativeSupportType,
    ) -> Option<bool> {
        let state = self.native_level_state(level, pos)?;
        use horizon_core::geometry::Direction;
        let direction = match face {
            NativeBlockFace::Down => Direction::Down,
            NativeBlockFace::Up => Direction::Up,
            NativeBlockFace::North => Direction::North,
            NativeBlockFace::South => Direction::South,
            NativeBlockFace::West => Direction::West,
            NativeBlockFace::East => Direction::East,
        };
        let b = self.res.behaviours();
        if !b.has_cached_support(state) {
            return None;
        }
        Some(match support {
            NativeSupportType::Full => b.is_face_sturdy(state, direction),
            NativeSupportType::Center => b.is_face_sturdy_center(state, direction),
            NativeSupportType::Rigid => b.is_face_sturdy_rigid(state, direction),
        })
    }

    /// Clip an empty-context ray against collider or outline shapes, without fluids.
    /// None on failed reads, dynamic shapes, invalid rays or traversal limits.
    /// API @API:level_shapes@.
    pub(super) fn native_level_clip(
        &self,
        level: LevelRef,
        from: [f64; 3],
        to: [f64; 3],
        blocks: NativeClipBlock,
        fluids: NativeClipFluid,
    ) -> Option<NativeClipHit> {
        use horizon_client::collisions::SceneCollisions;
        use horizon_core::geometry::Direction;
        use horizon_world::{
            collision_context::CollisionContext,
            world_ray::{RayBlockMode, RayFluidMode, clip_world},
        };
        let world = self.native_level_world(level)?;
        let mut rays = super::explosion::ColliderRays {
            scene: SceneCollisions {
                world,
                binding: self.collision,
                subject: None,
            },
            context: CollisionContext::empty(false),
            registry: self.pack.assets.registry,
            clip_data: Some(super::explosion::ClipData {
                facts: self.res.facts(),
                behaviours: self.res.behaviours(),
            }),
        };
        let mode = match blocks {
            NativeClipBlock::Collider => RayBlockMode::Collider,
            NativeClipBlock::Outline => RayBlockMode::Outline,
        };
        let fluid = match fluids {
            NativeClipFluid::None => RayFluidMode::None,
        };
        let result = clip_world(
            &mut rays,
            Vec3::new(from[0], from[1], from[2]),
            Vec3::new(to[0], to[1], to[2]),
            mode,
            fluid,
            4096,
        )
        .ok()?;
        let h = result.hit;
        let face = match h.direction {
            Direction::Down => NativeBlockFace::Down,
            Direction::Up => NativeBlockFace::Up,
            Direction::North => NativeBlockFace::North,
            Direction::South => NativeBlockFace::South,
            Direction::West => NativeBlockFace::West,
            Direction::East => NativeBlockFace::East,
        };
        Some(NativeClipHit {
            miss: result.miss,
            block: [h.block.x, h.block.y, h.block.z],
            face,
            location: [h.location.x, h.location.y, h.location.z],
            inside: h.inside,
            world_border: false,
        })
    }

    /// The entity's position (`Entity.position`): its feet, the middle of
    /// its box for a painting or an item frame. API @API:entity_handles@.
    pub(super) fn native_entity_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        self.target_position(self.native_entity_target(entity)?)
    }
    /// The block the entity's position is in (`Entity.blockPosition`). API
    /// @API:entity_handles@.
    pub(super) fn native_entity_block_position(&self, entity: EntityRef) -> Option<[i32; 3]> {
        self.native_entity_position(entity)
            .map(|p| p.map(|v| v.floor() as i32))
    }
    /// The entity's yaw and pitch in degrees (`Entity.getYRot`, `getXRot`); 0
    /// for the kinds that don't turn. API @API:entity_handles@.
    pub(super) fn native_entity_rotation(&self, entity: EntityRef) -> Option<[f32; 2]> {
        Some(self.target_rotation(self.native_entity_target(entity)?))
    }
    /// The entity's type (`Entity.getType`): `minecraft:zombie`. API
    /// @API:entity_handles@.
    pub(super) fn native_entity_kind(&self, entity: EntityRef) -> Option<String> {
        Some(self.target_type(self.native_entity_target(entity)?))
    }
    /// The entity's UUID (`Entity.getUUID`), the one it is saved with; an
    /// entity that has none yet gets it now, as a new vanilla entity does.
    /// API @API:entity_handles@.
    pub(super) fn native_entity_uuid(&mut self, entity: EntityRef) -> Option<u128> {
        let words = match self.native_entity_target(entity)? {
            Target::Player => return self.command_guest(self.lan_acting?).map(|g| g.uuid),
            Target::Guest(conn) => return self.command_guest(conn).map(|g| g.uuid),
            _ => self.uuid_for(entity.id()),
        };
        Some(uuid_from_words(words))
    }
    /// The level the entity is in (`Entity.level`). API @API:entity_handles@.
    pub(super) fn native_entity_level(&self, entity: EntityRef) -> Option<LevelRef> {
        let dim: Dim = self.target_level(self.native_entity_target(entity)?);
        Some(LevelRef::new(entity.epoch(), dim.id()))
    }
}

/// A saved UUID's four ints (`UUIDUtil.uuidFromIntArray`) as one number.
fn uuid_from_words(words: [i32; 4]) -> u128 {
    words
        .iter()
        .fold(0, |uuid, w| (uuid << 32) | u128::from(*w as u32))
}

#[cfg(test)]
pub(super) mod tests {
    pub(in crate::game) fn with_game(test: impl FnOnce(&mut super::Game<'_>)) {
        use super::super::{
            Audio, Game, Options, Progress, Settings, execution_role::ExecutionRole, load_resources,
        };
        use horizon_client::mod_api::NativeAuthority;

        let root = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
        let settings = Settings::default();
        let loaded = load_resources(
            root.clone(),
            None,
            &root,
            &settings,
            std::time::Instant::now(),
            false,
            true,
            &Progress::new(),
            &mut |_, _| unreachable!(),
        )
        .unwrap();
        let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
        options.resources = root.clone();
        options.execution_role = ExecutionRole::IntegratedServer;
        let mut game = Game::new(
            &loaded.resources,
            loaded.pack,
            loaded.gui.unwrap(),
            options,
            settings,
            Audio::silent(&root),
            None,
        )
        .unwrap();
        game.native_lifecycle
            .begin_open(NativeAuthority::IntegratedServer);
        test(&mut game);
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn chunk_handles_resolve_loaded_parked_and_stale_levels() {
        use horizon_client::{
            chunk::Chunk,
            mod_api::{ChunkStatus, Host, NativeAuthority},
        };
        use std::sync::Arc;
        with_game(|game| {
            let level = game.native_level(game.dim.id()).unwrap();
            game.world
                .chunks
                .insert((-2, 3), Arc::new(Chunk::empty(-2, 3)));
            let chunk = game.chunk(level, [-2, 3]).unwrap();
            assert_eq!(chunk.pos(), [-2, 3]);
            assert_eq!(chunk.level(), level);
            assert!(chunk.loaded(game));
            assert_eq!(chunk.status(game), Some(ChunkStatus::Full));
            assert!(game.chunk(level, [99, 99]).is_none());
            assert!(
                game.enter_command_level(super::super::Dim::Nether)
                    .is_some()
            );
            assert_eq!(chunk.status(game), Some(ChunkStatus::Full));
            assert_eq!(game.chunk(level, [-2, 3]), Some(chunk));
            assert!(
                game.enter_command_level(super::super::Dim::Overworld)
                    .is_some()
            );
            game.world.chunks.remove(&(-2, 3));
            assert!(!chunk.loaded(game));
            assert_eq!(chunk.status(game), None);
            game.world
                .chunks
                .insert((-2, 3), Arc::new(Chunk::empty(-2, 3)));
            game.native_lifecycle
                .begin_open(NativeAuthority::IntegratedServer);
            assert!(game.chunk(level, [-2, 3]).is_none());
            assert!(!chunk.loaded(game));
            assert_eq!(chunk.status(game), None);
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn equipping_mobs_by_hand_keeps_one_complete_stack() {
        use horizon_client::{items::ItemStack, mobs::Mob};
        use horizon_core::geometry::Vec3;
        with_game(|game| {
            game.creative = false;
            for (id, kind, item) in [
                (12, "minecraft:horse", "minecraft:iron_horse_armor"),
                (13, "minecraft:llama", "minecraft:red_carpet"),
                (14, "minecraft:happy_ghast", "minecraft:red_harness"),
                (15, "minecraft:nautilus", "minecraft:iron_nautilus_armor"),
            ] {
                let kind = game.res.mob_types().get(kind).unwrap().clone();
                let mut mob = Mob::new(id, kind, Vec3::ZERO, 0.0);
                mob.extra.tame = true;
                let mut held = ItemStack::new(item, 3);
                held.damage = 7;
                held.data_mut().custom_name = Some(horizon_world::nbt::Tag::String("kept".into()));
                game.inventory[game.selected] = Some(held.clone());
                let index = game.mobs.len();
                game.mobs.push(mob);
                let used = match id {
                    12 => game.interact_mount(index, item),
                    13 => game.interact_llama(index, item),
                    14 => game.interact_happy_ghast(index, item),
                    _ => game.interact_nautilus(index, item),
                };
                assert!(used, "{item}");
                assert_eq!(game.inventory[game.selected].as_ref().unwrap().count, 2);
                held.count = 1;
                assert_eq!(game.mobs[index].equipment_stacks()[6], Some(held), "{item}");
            }
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn sulfur_cube_absorbs_one_complete_stack_by_hand_and_pickup() {
        use horizon_client::{items::ItemStack, mobs::Mob};
        use horizon_core::geometry::Vec3;
        with_game(|game| {
            game.creative = false;
            let kind = game
                .res
                .mob_types()
                .get("minecraft:sulfur_cube")
                .unwrap()
                .clone();
            let item = kind.tempt.first().expect("a swallowable item").to_string();
            let mut held = ItemStack::new(&item, 3);
            held.data_mut().custom_name = Some(horizon_world::nbt::Tag::String("kept".into()));
            game.inventory[game.selected] = Some(held.clone());
            game.mobs.push(Mob::new(12, kind.clone(), Vec3::ZERO, 0.0));
            assert!(game.interact_sulfur_cube(0, &item));
            assert_eq!(game.inventory[game.selected].as_ref().unwrap().count, 2);
            held.count = 1;
            assert_eq!(game.mobs[0].equipment_stacks()[6], Some(held.clone()));
            game.mobs.clear();
            game.mobs.push(Mob::new(13, kind, Vec3::ZERO, 0.0));
            game.spawn_item(
                ItemStack {
                    count: 3,
                    ..held.clone()
                },
                [0.0, 0.0, 0.0],
                [0.0; 3],
                0,
            );
            game.sulfur_cube_pickups();
            assert_eq!(game.mobs[0].equipment_stacks()[6], Some(held));
            assert_eq!(game.dropped[0].stack.count, 2);
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn shearing_llama_and_happy_ghast_returns_full_body_stacks() {
        use horizon_client::{items::ItemStack, mobs::Mob};
        use horizon_core::geometry::Vec3;
        with_game(|game| {
            for (id, kind, item) in [
                (12, "minecraft:llama", "minecraft:red_carpet"),
                (13, "minecraft:happy_ghast", "minecraft:red_harness"),
            ] {
                let kind = game.res.mob_types().get(kind).unwrap().clone();
                let mut mob = Mob::new(id, kind, Vec3::ZERO, 0.0);
                mob.extra.tame = true;
                let mut stack = ItemStack::new(item, 3);
                stack.damage = 7;
                stack.data_mut().custom_name = Some(horizon_world::nbt::Tag::String("kept".into()));
                mob.extra.set_body_stack(Some(stack.clone()));
                let index = game.mobs.len();
                game.mobs.push(mob);
                let used = if item.ends_with("carpet") {
                    game.interact_llama(index, "minecraft:shears")
                } else {
                    game.interact_happy_ghast(index, "minecraft:shears")
                };
                assert!(used);
                assert!(game.dropped.iter().any(|drop| drop.stack == stack));
                assert!(game.mobs[index].extra.body.is_none());
            }
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn entities_in_returns_readable_handles_and_intersects_boxes() {
        use horizon_client::entities::ArmorStand;
        use horizon_client::mod_api::{LevelRef, NativeWorldEpoch};
        use horizon_core::geometry::Vec3;
        with_game(|game| {
            let level = game.native_level(game.dim.id()).unwrap();
            game.armor_stands
                .push(ArmorStand::new(0, Vec3::new(0.5, 0.0, 0.0), 0.0));
            assert!(game.native_entity(0).is_none());
            game.armor_stands
                .push(ArmorStand::new(12, Vec3::new(1.0, 0.0, 0.0), 0.0));
            let found = game
                .native_level_entities_in(level, [0.0, 0.0, -1.0], [0.8, 1.0, 1.0])
                .unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(game.native_entity_position(found[0]), Some([1.0, 0.0, 0.0]));
            assert_eq!(
                game.native_entity_kind(found[0]).as_deref(),
                Some("minecraft:armor_stand")
            );
            assert!(
                game.native_level_entities_in(level, [0.0, 0.0, -1.0], [0.75, 1.0, 1.0])
                    .unwrap()
                    .is_empty()
            );
            assert!(
                game.native_level_entities_in(
                    LevelRef::new(NativeWorldEpoch::new(0), game.dim.id()),
                    [0.0; 3],
                    [2.0; 3]
                )
                .is_none()
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn level_sturdy_uses_gameplay_support() {
        use horizon_client::{
            chunk::Chunk,
            mod_api::{
                LevelRef, NativeBlockFace as Face, NativeSupportType as Support, NativeWorldEpoch,
            },
        };
        use std::sync::Arc;
        with_game(|game| {
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let level = game.native_level(game.dim.id()).unwrap();
            let slab = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone_slab")
                .unwrap();
            game.world.set(0, 64, 0, slab).unwrap();
            assert_eq!(
                level.block_face_sturdy(game, [0, 64, 0], Face::Down, Support::Full),
                Some(true)
            );
            assert_eq!(
                level.block_face_sturdy(game, [0, 64, 0], Face::North, Support::Full),
                Some(false)
            );
            let wall = game
                .pack
                .assets
                .registry
                .default_state("minecraft:cobblestone_wall")
                .unwrap();
            game.world.set(0, 64, 0, wall).unwrap();
            assert_eq!(
                level.block_face_sturdy(game, [0, 64, 0], Face::Up, Support::Center),
                Some(true)
            );
            assert_eq!(
                level.block_face_sturdy(game, [16, 64, 0], Face::Down, Support::Full),
                None
            );
            assert_eq!(
                level.block_face_sturdy(game, [0, 10000, 0], Face::Down, Support::Full),
                None
            );
            assert!(
                game.enter_command_level(super::super::Dim::Nether)
                    .is_some()
            );
            assert_eq!(
                level.block_face_sturdy(game, [0, 64, 0], Face::Up, Support::Center),
                Some(true)
            );
            assert!(
                game.enter_command_level(super::super::Dim::Overworld)
                    .is_some()
            );
            let shulker = game
                .pack
                .assets
                .registry
                .default_state("minecraft:shulker_box")
                .unwrap();
            game.world.set(0, 64, 0, shulker).unwrap();
            assert_eq!(
                level.block_face_sturdy(game, [0, 64, 0], Face::Up, Support::Full),
                None
            );
            let stale = LevelRef::new(
                NativeWorldEpoch::new(level.epoch().get() + 1),
                level.dimension(),
            );
            assert_eq!(
                stale.block_face_sturdy(game, [0, 64, 0], Face::Down, Support::Full),
                None
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn level_clip_reads_parked_world_and_refuses_unloaded() {
        use horizon_client::{
            chunk::Chunk,
            mod_api::{
                NativeBlockFace as Face, NativeClipBlock as Block, NativeClipFluid as Fluid,
            },
        };
        use std::sync::Arc;
        with_game(|game| {
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let level = game.native_level(game.dim.id()).unwrap();
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            game.world.set(1, 64, 0, stone).unwrap();
            let from = [0.5, 64.5, 0.5];
            let to = [2.5, 64.5, 0.5];
            for mode in [Block::Collider, Block::Outline] {
                let hit = level.clip(game, from, to, mode, Fluid::None).unwrap();
                assert!(!hit.miss);
                assert_eq!(hit.block, [1, 64, 0]);
                assert_eq!(hit.face, Face::West);
                assert_eq!(hit.location, [1.0, 64.5, 0.5]);
                assert!(!hit.inside && !hit.world_border);
            }
            assert!(
                level
                    .clip(
                        game,
                        [15.5, 64.5, 0.5],
                        [17.5, 64.5, 0.5],
                        Block::Collider,
                        Fluid::None
                    )
                    .is_none()
            );
            let miss = level
                .clip(
                    game,
                    [0.5, 67.5, 0.5],
                    [2.5, 67.5, 0.5],
                    Block::Collider,
                    Fluid::None,
                )
                .unwrap();
            assert!(miss.miss);
            let inside = level
                .clip(
                    game,
                    [1.5, 64.5, 0.5],
                    [2.5, 64.5, 0.5],
                    Block::Collider,
                    Fluid::None,
                )
                .unwrap();
            assert!(inside.inside && !inside.miss);
            assert!(
                level
                    .clip(
                        game,
                        [f64::NAN, 64.0, 0.0],
                        to,
                        Block::Collider,
                        Fluid::None
                    )
                    .is_none()
            );
            let stale = horizon_client::mod_api::LevelRef::new(
                horizon_client::mod_api::NativeWorldEpoch::new(level.epoch().get() + 1),
                level.dimension(),
            );
            assert!(
                stale
                    .clip(game, from, to, Block::Collider, Fluid::None)
                    .is_none()
            );
            let cauldron = game
                .pack
                .assets
                .registry
                .default_state("minecraft:cauldron")
                .unwrap();
            game.world.set(0, 64, 2, cauldron).unwrap();
            let bowl = level
                .clip(
                    game,
                    [0.5, 65.27, 0.5],
                    [0.5, 64.85, 3.5],
                    Block::Outline,
                    Fluid::None,
                )
                .unwrap();
            assert!(!bowl.miss);
            assert_eq!(bowl.block, [0, 64, 2]);
            assert_eq!(bowl.face, Face::Up);
            assert!(
                game.enter_command_level(super::super::Dim::Nether)
                    .is_some()
            );
            assert_eq!(
                level
                    .clip(game, from, to, Block::Collider, Fluid::None)
                    .unwrap()
                    .block,
                [1, 64, 0]
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn living_attributes_are_unavailable_for_nonplayers() {
        use horizon_client::{entities::ArmorStand, mobs::Mob};
        use horizon_core::geometry::Vec3;
        with_game(|game| {
            let zombie = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs.push(Mob::new(12, zombie, Vec3::ZERO, 0.0));
            game.armor_stands.push(ArmorStand::new(13, Vec3::ZERO, 0.0));
            let mut dragon = super::super::dragon::Dragon::new(Vec3::ZERO, 0.0, 0);
            dragon.id = 14;
            game.dragon = Some(dragon);
            for id in [12, 13, 14] {
                let living = game.native_living(game.native_entity(id).unwrap()).unwrap();
                assert_eq!(living.attribute_base(game, "minecraft:max_health"), None);
                assert_eq!(living.attribute_value(game, "minecraft:max_health"), None);
            }
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn level_shapes_use_empty_context() {
        use super::super::{Audio, Game, Options, Progress, Settings, load_resources};
        use horizon_client::{
            chunk::Chunk,
            mod_api::{NativeAuthority, NativeBlockShapeKind},
        };
        use std::{sync::Arc, time::Instant};

        let root = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
        let settings = Settings::default();
        let loaded = load_resources(
            root.clone(),
            None,
            &root,
            &settings,
            Instant::now(),
            false,
            true,
            &Progress::new(),
            &mut |_, _| unreachable!(),
        )
        .unwrap();
        let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
        options.resources = root.clone();
        options.execution_role = super::super::execution_role::ExecutionRole::IntegratedServer;
        let mut game = Game::new(
            &loaded.resources,
            loaded.pack,
            loaded.gui.unwrap(),
            options,
            settings,
            Audio::silent(&root),
            None,
        )
        .unwrap();
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        game.native_lifecycle
            .begin_open(NativeAuthority::IntegratedServer);
        let level = game.native_level("minecraft:overworld").unwrap();
        assert!(
            game.enter_command_level(super::super::Dim::Nether)
                .is_some()
        );
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let stone = game
            .pack
            .assets
            .registry
            .default_state("minecraft:stone")
            .unwrap();
        game.world.set(0, 64, 0, stone).unwrap();
        assert!(
            game.enter_command_level(super::super::Dim::Overworld)
                .is_some()
        );
        let nether = game.native_level("minecraft:the_nether").unwrap();
        let cube = vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
        assert_eq!(
            game.native_level_block_shape(nether, [0, 64, 0], NativeBlockShapeKind::Outline),
            Some(cube.clone())
        );
        assert_eq!(
            game.native_level_block_shape(nether, [0, 64, 0], NativeBlockShapeKind::Collision),
            Some(cube)
        );
        let shape = |game: &mut Game<'_>, name: &str, kind| {
            let state = game.pack.assets.registry.default_state(name).unwrap();
            game.world.set(0, 64, 0, state).unwrap();
            game.native_level_block_shape(level, [0, 64, 0], kind)
                .unwrap()
        };
        game.inventory[game.selected] = Some(horizon_client::items::ItemStack {
            item: "minecraft:light".into(),
            count: 1,
            damage: 0,
            extra: None,
        });
        assert!(shape(&mut game, "minecraft:light", NativeBlockShapeKind::Outline).is_empty());
        assert_eq!(
            game.outline_boxes([0, 64, 0]),
            vec![[0.0, 64.0, 0.0, 1.0, 65.0, 1.0]]
        );
        game.inventory[game.selected].as_mut().unwrap().item = "minecraft:scaffolding".into();
        let scaffolding = shape(
            &mut game,
            "minecraft:scaffolding",
            NativeBlockShapeKind::Outline,
        );
        assert!(!scaffolding.is_empty());
        assert_ne!(scaffolding, vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]);
        assert_eq!(
            game.outline_boxes([0, 64, 0]),
            vec![[0.0, 64.0, 0.0, 1.0, 65.0, 1.0]]
        );
        assert_eq!(
            shape(
                &mut game,
                "minecraft:stone_slab",
                NativeBlockShapeKind::Outline
            ),
            vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]
        );
        assert_eq!(
            shape(
                &mut game,
                "minecraft:stone_slab",
                NativeBlockShapeKind::Collision
            ),
            vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]
        );
        let contains = |boxes: &[[f64; 6]], p: [f64; 3]| {
            boxes
                .iter()
                .any(|b| (0..3).all(|i| b[i] < p[i] && p[i] < b[i + 3]))
        };
        let stair = shape(
            &mut game,
            "minecraft:oak_stairs",
            NativeBlockShapeKind::Outline,
        );
        assert!(contains(&stair, [0.25, 0.25, 0.25]));
        let upper = [
            [0.25, 0.75, 0.25],
            [0.75, 0.75, 0.25],
            [0.25, 0.75, 0.75],
            [0.75, 0.75, 0.75],
        ];
        assert_eq!(
            upper.into_iter().filter(|p| contains(&stair, *p)).count(),
            2
        );
        let fence = shape(
            &mut game,
            "minecraft:oak_fence",
            NativeBlockShapeKind::Outline,
        );
        assert!(contains(&fence, [0.5, 0.5, 0.5]));
        assert!(!contains(&fence, [0.1, 0.5, 0.1]));
        let lectern = shape(
            &mut game,
            "minecraft:lectern",
            NativeBlockShapeKind::Collision,
        );
        assert!(contains(&lectern, [0.5, 0.5, 0.5]));
        assert!(!contains(&lectern, [0.1, 0.5, 0.1]));
    }

    #[test]
    fn uuid_words_are_most_significant_first() {
        let words = [0x0102_0304, -1, 0, 0x7fff_ffff];
        assert_eq!(
            super::uuid_from_words(words),
            0x0102_0304_ffff_ffff_0000_0000_7fff_ffff
        );
    }
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn level_block_writes_validate_and_change_world() {
        use super::super::{Audio, Game, Options, Progress, Settings, load_resources};
        use horizon_client::{
            chunk::Chunk,
            mod_api::{NativeAuthority, NativeBlockUpdateFlags},
        };
        use std::{sync::Arc, time::Instant};

        let root = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
        let settings = Settings::default();
        let loaded = load_resources(
            root.clone(),
            None,
            &root,
            &settings,
            Instant::now(),
            false,
            true,
            &Progress::new(),
            &mut |_, _| unreachable!(),
        )
        .unwrap();
        let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
        options.resources = root.clone();
        options.execution_role = super::super::execution_role::ExecutionRole::IntegratedServer;
        options.dedicated = Some(Vec::new());
        let mut game = Game::new(
            &loaded.resources,
            loaded.pack,
            loaded.gui.unwrap(),
            options,
            settings,
            Audio::silent(&root),
            None,
        )
        .unwrap();
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        game.native_lifecycle
            .begin_open(NativeAuthority::IntegratedServer);
        let level = game.native_level("minecraft:overworld").unwrap();
        let stone = game
            .pack
            .assets
            .registry
            .default_state("minecraft:stone")
            .unwrap();
        let chest = game
            .pack
            .assets
            .registry
            .default_state("minecraft:chest")
            .unwrap();
        let air = game
            .pack
            .assets
            .registry
            .default_state("minecraft:air")
            .unwrap();
        let flags = NativeBlockUpdateFlags::NEIGHBORS;
        assert!(game.native_level_set_block(level, [0, 64, 0], stone, flags));
        assert_eq!(game.world.block(0, 64, 0), stone);
        assert!(!game.native_level_set_block(level, [16, 64, 0], stone, flags));
        assert!(!game.native_level_set_block(level, [0, 10000, 0], stone, flags));
        let stale = horizon_client::mod_api::LevelRef::new(
            horizon_client::mod_api::NativeWorldEpoch::new(level.epoch().get() + 1),
            level.dimension(),
        );
        assert!(!game.native_level_set_block(stale, [0, 64, 0], stone, flags));
        assert!(!game.native_level_destroy_block(stale, [0, 64, 0], true));
        let fill_chest = |game: &mut Game<'_>| {
            let Some(super::super::BlockEntity::Chest(items)) =
                game.world.block_entities.get_mut(&(0, 64, 0))
            else {
                panic!("chest block entity missing")
            };
            items[0] = Some(horizon_client::items::ItemStack {
                item: "minecraft:diamond".into(),
                count: 3,
                damage: 0,
                extra: None,
            });
        };
        assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
        fill_chest(&mut game);
        assert!(game.native_level_set_block(level, [0, 64, 0], air, flags));
        assert_eq!(
            game.dropped
                .iter()
                .filter(|item| item.stack.item.as_ref() == "minecraft:diamond")
                .map(|item| item.stack.count)
                .sum::<u32>(),
            3
        );
        game.dropped.clear();
        assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
        fill_chest(&mut game);
        assert!(game.native_level_set_block(
            level,
            [0, 64, 0],
            air,
            NativeBlockUpdateFlags::SKIP_BLOCK_ENTITY_SIDE_EFFECTS
        ));
        assert!(!game.world.block_entities.contains_key(&(0, 64, 0)));
        assert!(game.dropped.is_empty());
        assert!(game.native_level_set_block(level, [0, 64, 0], stone, flags));
        assert!(game.native_level_destroy_block(level, [0, 64, 0], false));
        assert!(game.dropped.is_empty());
        assert_eq!(game.world.block(0, 64, 0), air);
        assert!(game.native_level_set_block(level, [0, 64, 0], stone, flags));
        assert!(game.native_level_destroy_block(level, [0, 64, 0], true));
        assert!(
            game.dropped
                .iter()
                .any(|item| item.stack.item.as_ref() == "minecraft:cobblestone")
        );
        assert!(!game.native_level_destroy_block(level, [0, 64, 0], true));
        game.dropped.clear();
        let copper = game
            .pack
            .assets
            .registry
            .default_state("minecraft:copper_chest")
            .unwrap();
        let exposed = game
            .pack
            .assets
            .registry
            .default_state("minecraft:exposed_copper_chest")
            .unwrap();
        assert!(game.native_level_set_block(level, [0, 64, 0], copper, flags));
        fill_chest(&mut game);
        assert!(game.native_level_set_block(level, [0, 64, 0], exposed, flags));
        let Some(super::super::BlockEntity::Chest(items)) =
            game.world.block_entities.get(&(0, 64, 0))
        else {
            panic!("copper chest entity missing")
        };
        assert_eq!(items[0].as_ref().unwrap().count, 3);
        assert!(game.dropped.is_empty());
        let sensor = game
            .pack
            .assets
            .registry
            .default_state("minecraft:sculk_sensor")
            .unwrap();
        assert!(game.native_level_set_block(level, [0, 64, 0], sensor, flags));
        assert!(matches!(
            game.world.block_entities.get(&(0, 64, 0)),
            Some(super::super::BlockEntity::Sculk(_))
        ));
        assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
        assert!(matches!(
            game.world.block_entities.get(&(0, 64, 0)),
            Some(super::super::BlockEntity::Chest(_))
        ));
        assert!(
            game.enter_command_level(super::super::Dim::Nether)
                .is_some()
        );
        assert!(!game.native_level_set_block(level, [0, 64, 0], stone, flags));
        assert!(!game.native_level_destroy_block(level, [0, 64, 0], true));
    }
}
