//! Entity and level handles for native server mods (docs/native-mods.md,
//! "Entity and level handles", #7387): `native_entity(id)` turns an event's
//! entity id into an `EntityRef`, and each `native_entity_*` helper reads the
//! entity a handle names. They read where the world is simulated: on a client,
//! with no world open, or for a handle of an earlier open, they return `None`.
//! The entities are those of the level in place (the one a hook runs in) and
//! the server's players, whatever their level.
use super::{Dim, Game, commands::Target};
use horizon_client::mod_api::{EntityRef, LevelRef, NativeBlockShapeKind};

impl Game<'_> {
    /// What a handle names now, or None for a stale or foreign one.
    fn native_entity_target(&self, entity: EntityRef) -> Option<Target> {
        if self.remote.is_some() || self.native_lifecycle.epoch() != Some(entity.epoch()) {
            return None;
        }
        let id = i32::try_from(entity.id()).ok()?;
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
    /// Block-local outline or collision boxes with an empty context. Interaction,
    /// visual, occlusion and support shapes are not exposed. Returns None on a
    /// client, for a stale handle, or for a parked level. Unloaded blocks read
    /// as air; collision errors return no boxes, like `native_block_collision`.
    /// API @API:level_shapes@.
    pub(super) fn native_level_block_shape(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        kind: NativeBlockShapeKind,
    ) -> Option<Vec<[f64; 6]>> {
        if self.remote.is_some()
            || self.native_lifecycle.epoch() != Some(level.epoch())
            || self.dim.id() != level.dimension()
        {
            return None;
        }
        Some(match kind {
            NativeBlockShapeKind::Outline => self
                .outline_boxes_in_context(pos, false)
                .into_iter()
                .map(|b| std::array::from_fn(|i| b[i] - f64::from(pos[i % 3])))
                .collect(),
            NativeBlockShapeKind::Collision => self.native_block_collision(pos),
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
mod tests {
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
}
