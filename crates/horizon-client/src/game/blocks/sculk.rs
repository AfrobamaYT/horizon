//! Game events and sculk on the game side: dispatching events to sculk
//! sensors, shriekers and wardens (`GameEventDispatcher`), the vibration
//! ticker, redstone output, resonance, shrieks and warden summoning
//! (`SculkShriekerBlockEntity`, `SpawnUtil`), catalysts feeding the sculk
//! spreader from deaths, and the darkness wardens and shriekers cast.
use super::{Game, Result};
use horizon_client::actions::Action;
use horizon_client::{
    containers::BlockEntity,
    effects::{EffectInstance, PlayerEffects},
    fast_hash::{FxHashMap, FxHashSet},
    mobs::{Mob, Species, Who, allay, warden},
    particles::{ParticleOptions, ParticleType},
    sculk::{self, Context, Info, SculkEntity, SculkKind, WardenSpawnTracker},
};
use horizon_client_core::who::EventSource;
use horizon_core::{
    geometry::{Aabb, Direction, Vec3},
    position::BlockPos,
    random::RandomSource,
};
use horizon_world::registry::BlockStateId;
use mod_api::particles::PositionSource;
use std::collections::HashSet;

pub const HELP: &[&str] = &[
    "sculk                          sculk listeners, the warning level and the darkness blend",
    "sculk charge X Y Z AMOUNT      a mob dying there with that much experience (the nearest catalyst takes it)",
    "sculk warn LEVEL               set the player's warden warning level",
    "sculk dig                      end the minute of calm of every warden",
];

/// The sculk block kinds by state id, built once.
#[derive(Default)]
struct Tables {
    built: bool,
    /// 0 none, 1 sensor, 2 calibrated sensor, 3 shrieker, 4 catalyst.
    kinds: Vec<u8>,
    occludes: Vec<bool>,
    dampens: Vec<bool>,
    resonator: Vec<bool>,
    /// The `minecraft:climbable` tag, and the two redstone ores: read for
    /// every mob on every tick.
    climbable: Vec<bool>,
    redstone_ore: Vec<bool>,
}
/// `SculkSensorBlock.getAnalogOutputSignal`: the last vibration's frequency
/// while the sensor is active.
pub(super) fn sensor_signal(
    world: &horizon_client::world::World,
    behaviours: &horizon_world::level::Behaviours,
    state: BlockStateId,
    pos: BlockPos,
) -> Option<i32> {
    match world.block_entities.get(&(pos.x, pos.y, pos.z)) {
        Some(BlockEntity::Sculk(e))
            if matches!(e.kind, SculkKind::Sensor | SculkKind::Calibrated) =>
        {
            Some(if behaviours.sculk_sensor_active(state) {
                e.last_frequency
            } else {
                0
            })
        }
        _ => None,
    }
}
/// `Entity.moveDist` / `nextStep` and the last state, per entity.
#[derive(Clone, Copy, Debug)]
struct Steps {
    move_dist: f32,
    next_step: f32,
    last: [f64; 3],
    on_ground: bool,
    in_water: bool,
}
#[derive(Default)]
pub(super) struct PlayerEvents {
    steps: Option<Steps>,
    last_health: Option<f32>,
    mount: Option<(u32, [f64; 3])>,
}

#[derive(Default)]
pub(super) struct SculkState {
    /// The player's warning level toward a warden.
    pub tracker: WardenSpawnTracker,
    tables: Tables,
    /// The sculk block entities in the order they were added.
    index: Index,
    pub known: HashSet<(i32, i32)>,
    /// The chunk map's membership revision `known` was taken at.
    pub known_revision: Option<u64>,
    // Keyed by mob ids the server gives out: the fast hasher is safe.
    steps: FxHashMap<Who, Steps>,
    /// Items that have fallen since last touching the ground or water.
    falling_items: HashSet<u32>,
    /// The vehicle each rider had last tick and where it stood.
    mounts: FxHashMap<Who, (u32, [f64; 3])>,
    damaged: FxHashMap<u32, i64>,
    /// Set while the mobs' events are posted (`tick_mob_sculk_events`).
    mob_slots: Option<MobSlots>,
    last_health: Option<f32>,
    /// The darkness effect's blend factor, previous and current tick.
    pub darkness: (f32, f32),
    /// Who causes the block changes being processed (the player's actions).
    pub actor: Option<Who>,
    ticks: u32,
    pub tracker_loaded: bool,
}

/// How far from a warden's or an allay's feet an event may be heard: their
/// listeners reach 16 blocks from the eye's block (`warden::listen`,
/// `allay::listen`), the eye stands under 3 blocks up, and flooring to blocks
/// adds under 2. Past this they do nothing with it.
const LISTENING_MOB_REACH: f64 = 24.0;

fn dist_sqr_f(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

/// Where each mob is in `Game::mobs` while the mobs' events are posted, for
/// the thousands of lookups by id they make, and whether any mob listens.
struct MobSlots {
    /// The first slot of each id, as a scan would find.
    by_id: FxHashMap<u32, usize>,
    /// The wardens and allays (the mobs that listen): slot and id.
    listeners: Vec<(usize, u32)>,
    /// `Game::next_entity` when they were taken: unchanged, no mob joined.
    next_entity: u32,
}

impl SculkState {
    pub(super) fn swap_player(&mut self, other: &mut PlayerEvents) {
        let steps = self.steps.remove(&Who::Player);
        if let Some(guest) = other.steps.take() {
            self.steps.insert(Who::Player, guest);
        }
        other.steps = steps;
        let mount = self.mounts.remove(&Who::Player);
        if let Some(guest) = other.mount.take() {
            self.mounts.insert(Who::Player, guest);
        }
        other.mount = mount;
        std::mem::swap(&mut self.last_health, &mut other.last_health);
    }

    /// Exchange the level-held parts (its sculk block entities and the
    /// chunks scanned for them) with another level's (game/world/levels.rs).
    pub(super) fn swap_level(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.index, &mut other.index);
        std::mem::swap(&mut self.known, &mut other.known);
    }
}
/// An insertion-ordered set of block positions (`Level.blockEntityTickers`
/// keeps the order block entities were added in).
#[derive(Default)]
struct Index {
    order: Vec<(i32, i32, i32)>,
    set: HashSet<(i32, i32, i32)>,
    /// The positions by 32-block cell, each with its rank: ranks only grow,
    /// so sorting by them gives `order`'s order. Every mob's step posts a
    /// game event, and the listeners near it are read from at most eight
    /// cells instead of the whole level's.
    cells: FxHashMap<(i32, i32, i32), Vec<(u64, (i32, i32, i32))>>,
    next: u64,
}
fn index_cell(key: (i32, i32, i32)) -> (i32, i32, i32) {
    (key.0 >> 5, key.1 >> 5, key.2 >> 5)
}
impl Index {
    fn insert(&mut self, key: (i32, i32, i32)) {
        if self.set.insert(key) {
            self.order.push(key);
            self.cells
                .entry(index_cell(key))
                .or_default()
                .push((self.next, key));
            self.next += 1;
        }
    }
    fn remove(&mut self, key: &(i32, i32, i32)) {
        if self.set.remove(key) {
            self.order.retain(|k| k != key);
            let cell = index_cell(*key);
            if let Some(list) = self.cells.get_mut(&cell) {
                list.retain(|(_, k)| k != key);
                if list.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }
    /// The positions whose `dist_sqr` to `at` is at most `radius` squared,
    /// in `order`'s order: what filtering `iter` gives.
    fn within(&self, at: [i32; 3], radius: i32) -> Vec<(i32, i32, i32)> {
        let limit = i64::from(radius * radius);
        let span = |v: i32| {
            let cell = |w: i64| (w >> 5).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            cell(i64::from(v) - i64::from(radius))..=cell(i64::from(v) + i64::from(radius))
        };
        let mut found: Vec<(u64, (i32, i32, i32))> = Vec::new();
        for x in span(at[0]) {
            for y in span(at[1]) {
                for z in span(at[2]) {
                    if let Some(list) = self.cells.get(&(x, y, z)) {
                        found.extend(list.iter().filter(|(_, k)| dist_sqr(at, *k) <= limit));
                    }
                }
            }
        }
        found.sort_unstable_by_key(|(rank, _)| *rank);
        found.into_iter().map(|(_, k)| k).collect()
    }
    fn contains(&self, key: &(i32, i32, i32)) -> bool {
        self.set.contains(key)
    }
    fn iter(&self) -> impl Iterator<Item = &(i32, i32, i32)> {
        self.order.iter()
    }
    fn retain(&mut self, mut keep: impl FnMut(&(i32, i32, i32)) -> bool) {
        self.order.retain(|k| keep(k));
        self.set.retain(|k| keep(k));
        self.cells.retain(|_, list| {
            list.retain(|(_, k)| keep(k));
            !list.is_empty()
        });
    }
}
const KIND_SENSOR: u8 = 1;
const KIND_CALIBRATED: u8 = 2;
const KIND_SHRIEKER: u8 = 3;
const KIND_CATALYST: u8 = 4;

fn center(p: (i32, i32, i32)) -> [f64; 3] {
    [
        f64::from(p.0) + 0.5,
        f64::from(p.1) + 0.5,
        f64::from(p.2) + 0.5,
    ]
}
fn block_of(p: [f64; 3]) -> [i32; 3] {
    [
        p[0].floor() as i32,
        p[1].floor() as i32,
        p[2].floor() as i32,
    ]
}
fn dist_sqr(a: [i32; 3], b: (i32, i32, i32)) -> i64 {
    let d = |x: i32, y: i32| i64::from(x) - i64::from(y);
    d(a[0], b.0).pow(2) + d(a[1], b.1).pow(2) + d(a[2], b.2).pow(2)
}

impl Game<'_> {
    /// Run `f` with the player as the cause of the block changes it makes.
    pub(super) fn with_player_actor<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let prior = self.sculk.actor.replace(Who::Player);
        let out = f(self);
        self.sculk.actor = prior;
        out
    }

    /// The `sculk` control command: listeners and their vibrations, or a
    /// test charge for the nearest catalyst.
    pub(super) fn sculk_command(&mut self, words: &[&str]) -> Result<serde_json::Value> {
        match words.get(1).copied() {
            Some("charge") => {
                let num = |i: usize| -> Result<f64> {
                    Ok(words.get(i).ok_or("missing argument")?.parse::<f64>()?)
                };
                let at = [num(2)?, num(3)?, num(4)?];
                let amount = num(5)? as u32;
                let taken = self.catalyst_takes_experience(at, |_| amount);
                Ok(serde_json::json!({ "ok": true, "taken": taken }))
            }
            Some("dig") => {
                // Let every warden's minute of calm run out.
                for m in self
                    .mobs
                    .iter_mut()
                    .filter(|m| m.species() == Species::Warden)
                {
                    warden::expire_dig_cooldown(m);
                }
                Ok(serde_json::json!({ "ok": true }))
            }
            Some("warn") => {
                let level = words.get(2).ok_or("missing level")?.parse::<i32>()?;
                self.sculk.tracker.set_warning_level(level);
                Ok(serde_json::json!({ "ok": true }))
            }
            _ => Ok(self.sculk_status()),
        }
    }
    /// The `sculk` control command's listing.
    pub(super) fn sculk_status(&self) -> serde_json::Value {
        let listeners: Vec<serde_json::Value> = self
            .sculk
            .index
            .iter()
            .filter_map(|key| match self.world.block_entities.get(key) {
                Some(BlockEntity::Sculk(e)) => Some(serde_json::json!({
                    "pos": [key.0, key.1, key.2],
                    "kind": e.id(),
                    "current": e.vibration.current.map(|i| i.event),
                    "travel": e.vibration.travel,
                    "last_frequency": e.last_frequency,
                    "warning_level": e.warning_level,
                    "cursors": e.spreader.cursors.len(),
                })),
                _ => None,
            })
            .collect();
        serde_json::json!({
            "listeners": listeners,
            "warning_level": self.sculk.tracker.warning_level,
            "cooldown": self.sculk.tracker.cooldown_ticks,
            "darkness": self.sculk.darkness.1,
            "chunks_known": self.sculk.known.len(),
        })
    }
    /// `WardenSpawnTrackerCommand`: clear the player's tracker or set its
    /// warning level.
    pub(super) fn cmd_warden_spawn_tracker(&mut self, level: Option<i32>) {
        use horizon_client::text::Text;
        let key = match level {
            Some(level) => {
                self.sculk.tracker.set_warning_level(level);
                "commands.warden_spawn_tracker.set.success.single"
            }
            None => {
                self.sculk.tracker.reset();
                "commands.warden_spawn_tracker.clear.success.single"
            }
        };
        if self
            .ui
            .rules
            .bool_at(horizon_client::gamerules::vanilla::SEND_COMMAND_FEEDBACK)
        {
            self.system_message(Text::translate(
                key,
                &[Text::literal(super::chat::PLAYER_NAME)],
            ));
        }
    }
    /// `ServerPlayer`'s `warden_spawn_tracker` from the player data.
    pub(super) fn load_warden_tracker(&mut self) {
        if self.sculk.tracker_loaded {
            return;
        }
        self.sculk.tracker_loaded = true;
        if let Some(tag) = self
            .level
            .player_extra
            .as_ref()
            .and_then(|p| p.get("warden_spawn_tracker"))
        {
            self.sculk.tracker = WardenSpawnTracker::from_nbt(tag);
        }
    }
    /// Write the tracker into the player data being saved.
    pub(super) fn save_warden_tracker(&mut self) {
        if !self.sculk.tracker_loaded {
            return;
        }
        let tag = self.sculk.tracker.to_nbt();
        let extra = self
            .level
            .player_extra
            .get_or_insert_with(|| horizon_core::nbt::Tag::Compound(Default::default()));
        if let horizon_core::nbt::Tag::Compound(map) = extra {
            map.insert("warden_spawn_tracker".into(), tag);
        }
    }
    /// Who is causing the block changes being processed.
    pub(super) fn sculk_actor(&self) -> Option<Who> {
        self.sculk.actor
    }

    // ---- setup and the block entity index ----

    pub(super) fn sculk_prepare(&mut self) {
        if self.sculk.tables.built {
            return;
        }
        let registry = self.pack.assets.registry;
        let n = registry.len();
        let tag = |name: &str| {
            self.res
                .items()
                .block_tags
                .get(name)
                .cloned()
                .unwrap_or_default()
        };
        let (wool, dampening, resonators, climbable) = (
            tag("minecraft:occludes_vibration_signals"),
            tag("minecraft:dampens_vibrations"),
            tag("minecraft:vibration_resonators"),
            tag("minecraft:climbable"),
        );
        let mut t = Tables {
            built: true,
            kinds: vec![0; n],
            occludes: vec![false; n],
            dampens: vec![false; n],
            resonator: vec![false; n],
            climbable: vec![false; n],
            redstone_ore: vec![false; n],
        };
        for i in 0..n {
            let facts = self.res.facts().get(BlockStateId(i as u32));
            let name: &str = &facts.name;
            t.kinds[i] = match name {
                "minecraft:sculk_sensor" => KIND_SENSOR,
                "minecraft:calibrated_sculk_sensor" => KIND_CALIBRATED,
                "minecraft:sculk_shrieker" => KIND_SHRIEKER,
                "minecraft:sculk_catalyst" => KIND_CATALYST,
                _ => 0,
            };
            t.occludes[i] = wool.contains(name);
            t.dampens[i] = dampening.contains(name);
            t.resonator[i] = resonators.contains(name);
            t.climbable[i] = climbable.contains(name);
            t.redstone_ore[i] = matches!(
                name,
                "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore"
            );
        }
        self.world.watched = t.kinds.iter().map(|k| *k != 0).collect();
        self.sculk.tables = t;
    }
    fn kind_of_state(&self, state: u32) -> u8 {
        self.sculk
            .tables
            .kinds
            .get(state as usize)
            .copied()
            .unwrap_or(0)
    }
    fn state_is(&self, table: &[bool], state: BlockStateId) -> bool {
        table.get(state.0 as usize).copied().unwrap_or(false)
    }
    fn entity_for_kind(kind: u8) -> Option<SculkEntity> {
        Some(SculkEntity::new(match kind {
            KIND_SENSOR => SculkKind::Sensor,
            KIND_CALIBRATED => SculkKind::Calibrated,
            KIND_SHRIEKER => SculkKind::Shrieker,
            KIND_CATALYST => SculkKind::Catalyst,
            _ => return None,
        }))
    }
    /// Find the sculk blocks of chunks that just became resident, and apply
    /// the block writes recorded since the last tick.
    pub(super) fn sculk_sync_index(&mut self) {
        // Chunks that arrived or left (every half second is plenty).
        // Nothing to do while no chunk came or went.
        let revision = self.world.chunks.membership_revision();
        if self.sculk.ticks.is_multiple_of(10) && self.sculk.known_revision != Some(revision) {
            self.sculk.known_revision = Some(revision);
            let loaded: HashSet<(i32, i32)> = self.world.chunks.keys().copied().collect();
            let arrived: Vec<(i32, i32)> = loaded.difference(&self.sculk.known).copied().collect();
            for key in arrived {
                self.sculk_scan_chunk(key);
            }
            // The blocks of every chunk that left, in one pass: a player
            // leaving unloads thousands of chunks at once.
            let gone: HashSet<(i32, i32)> = self.sculk.known.difference(&loaded).copied().collect();
            if !gone.is_empty() {
                self.sculk
                    .index
                    .retain(|p| !gone.contains(&(p.0 >> 4, p.2 >> 4)));
            }
            self.sculk.known = loaded;
        }
        let writes = std::mem::take(&mut self.world.watched_writes);
        for (key, old, new) in writes {
            let (old_kind, new_kind) = (self.kind_of_state(old), self.kind_of_state(new));
            if old_kind == new_kind {
                continue;
            }
            // SculkShriekerBlockEntity.preRemoveSideEffects.
            if old_kind == KIND_SHRIEKER
                && let Some(BlockEntity::Sculk(e)) = self.world.block_entities.get(&key)
                && self.shrieker_shrieking(BlockStateId(old))
            {
                let warning = e.warning_level;
                self.shrieker_respond(key, BlockStateId(old), warning);
            }
            if matches!(
                self.world.block_entities.get(&key),
                Some(BlockEntity::Sculk(_))
            ) {
                self.world.block_entities.remove(&key);
            }
            self.sculk.index.remove(&key);
            if let Some(e) = Self::entity_for_kind(new_kind) {
                self.world
                    .block_entities
                    .insert(key, BlockEntity::Sculk(Box::new(e)));
                self.sculk.index.insert(key);
            }
        }
    }
    fn sculk_scan_chunk(&mut self, key: (i32, i32)) {
        let Some(chunk) = self.world.chunks.get(&key).cloned() else {
            return;
        };
        let mut found = Vec::new();
        for (s, section) in chunk.sections.iter().enumerate() {
            let y0 = self.world.bottom_y() + s as i32 * 16;
            // Without a sculk state in its palette, no block of the section
            // is one: most sections are passed over without reading a block.
            if !section.blocks.maybe_has(|v| self.kind_of_state(v) != 0) {
                continue;
            }
            if section.blocks.uniform().is_some() {
                // A uniform section of sculk blocks does not exist.
                continue;
            }
            let mut a = [0u32; 4096];
            section.unpack(&mut a);
            for (i, v) in a.iter().enumerate() {
                if self.kind_of_state(*v) != 0 {
                    let (x, z, y) = (i & 15, (i >> 4) & 15, i >> 8);
                    found.push((
                        key.0 * 16 + x as i32,
                        y0 + y as i32,
                        key.1 * 16 + z as i32,
                        *v,
                    ));
                }
            }
        }
        for (x, y, z, state) in found {
            let pos = (x, y, z);
            if !matches!(
                self.world.block_entities.get(&pos),
                Some(BlockEntity::Sculk(_))
            ) && let Some(e) = Self::entity_for_kind(self.kind_of_state(state))
            {
                self.world
                    .block_entities
                    .insert(pos, BlockEntity::Sculk(Box::new(e)));
            }
            self.sculk.index.insert(pos);
        }
    }
    fn sculk_entity(&mut self, key: (i32, i32, i32)) -> Option<&mut SculkEntity> {
        match self.world.block_entities.get_mut(&key) {
            Some(BlockEntity::Sculk(e)) => Some(e),
            _ => None,
        }
    }

    // ---- contexts ----

    fn state_dampens(&self, state: Option<BlockStateId>) -> bool {
        state.is_some_and(|s| self.state_is(&self.sculk.tables.dampens, s))
    }
    /// `GameEvent.Context.of(player, state)`.
    pub(super) fn player_context(&self, state: Option<BlockStateId>) -> Context {
        let (dead, creative) = self
            .player
            .as_ref()
            .map_or((true, true), |p| (p.health.dead_or_dying(), self.creative));
        Context {
            source: Some(Who::Player.into()),
            owner: None,
            affected_state: state,
            dampening_state: self.state_dampens(state),
            spectator: self.spectator(),
            stepping_carefully: self.keys.held(Action::Sneak),
            dampens: false,
            targetable: !dead && !creative && !self.spectator(),
            living: true,
            player: true,
        }
    }
    /// `GameEvent.Context.of(mob, state)`.
    pub(super) fn mob_context(&self, id: u32, state: Option<BlockStateId>) -> Context {
        self.context_of_mob(self.mob_by_id(id), id, state)
    }
    /// The first mob with this id, from the slots taken for the mobs' events
    /// while the mob is still there (mobs are only appended meanwhile), else
    /// by a scan.
    fn mob_by_id(&self, id: u32) -> Option<&Mob> {
        if let Some(slots) = &self.sculk.mob_slots
            && let Some(&i) = slots.by_id.get(&id)
            && self.mobs.get(i).is_some_and(|m| m.id == id)
        {
            return self.mobs.get(i);
        }
        self.mobs.iter().find(|m| m.id == id)
    }
    fn context_of_mob(&self, mob: Option<&Mob>, id: u32, state: Option<BlockStateId>) -> Context {
        Context {
            source: Some(Who::Mob(id).into()),
            owner: None,
            affected_state: state,
            dampening_state: self.state_dampens(state),
            spectator: false,
            stepping_carefully: false,
            dampens: mob.is_some_and(|m| m.species() == Species::Warden),
            targetable: mob.is_some_and(|m| m.alive() && m.species() != Species::Warden),
            living: true,
            player: mob.is_some_and(|m| m.rider.is_some()),
        }
    }
    pub(super) fn who_context(&self, who: Option<Who>, state: Option<BlockStateId>) -> Context {
        match who {
            Some(Who::Player) => self.player_context(state),
            Some(Who::Mob(id)) => self.mob_context(id, state),
            None => Context {
                affected_state: state,
                dampening_state: self.state_dampens(state),
                ..Context::default()
            },
        }
    }

    /// `GameEvent.Context.of(entity, state)` for a non-living entity.
    pub(super) fn entity_event_context(&self, id: u32, state: Option<BlockStateId>) -> Context {
        let dampens = self
            .dropped
            .iter()
            .find(|item| item.id == id)
            .is_some_and(|item| {
                world_items!(self)
                    .item_tags
                    .get("minecraft:dampens_vibrations")
                    .is_some_and(|tag| tag.contains(item.stack.item.as_ref()))
            });
        Context {
            source: Some(EventSource::Entity(id)),
            affected_state: state,
            dampening_state: self.state_dampens(state),
            dampens,
            ..Context::default()
        }
    }

    fn event_source_context(
        &self,
        source: Option<EventSource>,
        state: Option<BlockStateId>,
    ) -> Context {
        match source {
            Some(EventSource::Entity(id)) => self.entity_event_context(id, state),
            _ => self.who_context(source.and_then(EventSource::actor), state),
        }
    }

    // ---- posting ----

    /// `Level.gameEvent`: tell the listeners within reach.
    pub(super) fn post_game_event(&mut self, event: &'static str, pos: [f64; 3], ctx: Context) {
        #[cfg(test)]
        TEST_POSTED_EVENTS.with(|events| {
            if let Some(events) = events.borrow_mut().as_mut() {
                events.push((event, pos, ctx.source.and_then(|source| source.actor())));
            }
        });
        #[cfg(test)]
