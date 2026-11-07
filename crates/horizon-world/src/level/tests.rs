//! Behaviour tests over an in-memory level. They need the prepared client
//! resources (`HORIZON_RESOURCES`, or `.local/horizon-client/<version>` at the
//! repository root) and are skipped when those are absent.
use super::*;
use crate::chunk::BlockState;
use horizon_core::random::RandomSource;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::OnceLock,
};

struct Fixture {
    registry: BlockRegistry,
    behaviours: Behaviours,
}

fn fixture() -> Option<&'static Fixture> {
    static FIXTURE: OnceLock<Option<Fixture>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let root = std::env::var_os("HORIZON_RESOURCES")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../.local/horizon-client")
                        .join(horizon_core::VANILLA_VERSION)
                });
            if !root.join("block-behaviours.json").exists() {
                eprintln!("skipping level tests: no resources at {}", root.display());
                return None;
            }
            let registry = data::load_registry(&root).expect("registry");
            let behaviours = Behaviours::load(&registry, &root).expect("behaviours");
            Some(Fixture {
                registry,
                behaviours,
            })
        })
        .as_ref()
}

pub(crate) struct TestLevel {
    blocks: HashMap<BlockPos, BlockStateId>,
    air: BlockStateId,
    random: LegacyRandom,
    pub time: i64,
    pub day_time: i64,
    pub events: Vec<LevelEvent>,
    pub block_light: HashMap<BlockPos, i32>,
    /// `Level.isClientSide`.
    pub client_side: bool,
    /// Positions that refuse fluids, fire and piston moves, as a land claim would.
    pub guarded: HashSet<BlockPos>,
    /// What `adjust_signal` answers at a position: the weak and direct signal,
    /// and the only direction they go in, if any.
    pub signals: HashMap<BlockPos, (i32, i32, Option<Direction>)>,
    /// Overrides the conductor check for individual signal queries.
    pub weak_power: HashMap<BlockPos, bool>,
    pub weak_power_directions: std::cell::RefCell<Vec<Direction>>,
    /// What `adjust_analog_output` answers at a position, for the states in
    /// `analog_states` (`adjust_has_analog_output`).
    pub analog: HashMap<BlockPos, i32>,
    pub analog_states: HashSet<BlockStateId>,
    /// What `adjust_should_redstone_wire_connect_to` answers at a position: whether
    /// redstone wire connects there.
    pub wire_connect: HashMap<BlockPos, bool>,
    /// What `adjust_connects_to` answers for a kind of connection and a
    /// neighbour state: whether the fence, wall or bars connect toward it.
    pub connect: HashMap<(ConnectionKind, BlockStateId), bool>,
    /// The kinds and directions `adjust_connects_to` was asked about.
    pub connection_asks: std::cell::RefCell<Vec<(ConnectionKind, Direction)>>,
    /// What `before_fire_odds` answers for a state.
    pub fire_odds: HashMap<BlockStateId, (i32, i32)>,
    /// What `before_smoke_source` answers for a state.
    pub smoke_sources: HashMap<BlockStateId, bool>,
    /// What `recheck_container_openers` answers at a position: the opener
    /// count before and after the recount.
    pub openers: HashMap<BlockPos, (i32, i32)>,
    /// Every position whose container openers were recounted, in order.
    pub opener_rechecks: Vec<BlockPos>,
    /// What `note_block_play` answers: the instrument index and note it
    /// plays instead, or None to stop the note.
    pub note_answer: Option<Option<(u8, i32)>>,
    /// `by_player` of every note the level was asked about, in order.
    pub note_players: std::cell::RefCell<Vec<bool>>,
    /// `allow_portal_spawn` refuses every portal.
    pub refuse_portals: bool,
    /// `ShulkerBoxBlockEntity.isClosed` at a position: `true` for a shut lid,
    /// `false` for an open box. Positions not listed have no block entity.
    pub shulker_closed: HashMap<BlockPos, bool>,
    /// The containers `container_open` reports as open.
    pub opened: HashSet<BlockPos>,
}
impl TestLevel {
    fn new(b: &Behaviours) -> Self {
        Self {
            blocks: HashMap::new(),
            air: b.air(),
            random: LegacyRandom::new(42),
            signals: HashMap::new(),
            weak_power: HashMap::new(),
            weak_power_directions: std::cell::RefCell::new(Vec::new()),
            connection_asks: std::cell::RefCell::new(Vec::new()),
            analog: HashMap::new(),
            analog_states: HashSet::new(),
            wire_connect: HashMap::new(),
            connect: HashMap::new(),
            time: 0,
            day_time: 0,
            events: Vec::new(),
            block_light: HashMap::new(),
            client_side: false,
            guarded: HashSet::new(),
            fire_odds: HashMap::new(),
            smoke_sources: HashMap::new(),
            openers: HashMap::new(),
            opener_rechecks: Vec::new(),
            note_answer: None,
            note_players: std::cell::RefCell::new(Vec::new()),
            refuse_portals: false,
            shulker_closed: HashMap::new(),
            opened: HashSet::new(),
        }
    }
}
impl LevelAccess for TestLevel {
    fn container_open(&self, pos: BlockPos) -> bool {
        self.opened.contains(&pos)
    }
    fn recheck_container_openers(&mut self, pos: BlockPos) -> Option<(i32, i32)> {
        self.opener_rechecks.push(pos);
        self.openers.get(&pos).copied()
    }
    fn shulker_is_closed(&self, pos: BlockPos) -> Option<bool> {
        self.shulker_closed.get(&pos).copied()
    }
    fn adjust_has_analog_output(&self, state: BlockStateId, vanilla: bool) -> bool {
        vanilla || self.analog_states.contains(&state)
    }
    fn has_weak_power_hook(&self) -> bool {
        !self.weak_power.is_empty()
    }
    fn adjust_should_check_weak_power(
        &self,
        pos: BlockPos,
        _state: BlockStateId,
        direction: Direction,
        vanilla: bool,
    ) -> bool {
        self.weak_power_directions.borrow_mut().push(direction);
        self.weak_power.get(&pos).copied().unwrap_or(vanilla)
    }
    fn has_redstone_wire_connection_hook(&self) -> bool {
        !self.wire_connect.is_empty()
    }
    fn adjust_should_redstone_wire_connect_to(
        &self,
        pos: BlockPos,
        _state: BlockStateId,
        _direction: Option<Direction>,
        vanilla: bool,
    ) -> bool {
        self.wire_connect.get(&pos).copied().unwrap_or(vanilla)
    }
    fn has_connection_hook(&self) -> bool {
        !self.connect.is_empty()
    }
    fn adjust_connects_to(
        &self,
        kind: ConnectionKind,
        _pos: BlockPos,
        _state: BlockStateId,
        neighbour: BlockStateId,
        direction: Direction,
        _face_solid: bool,
        vanilla: bool,
    ) -> bool {
        self.connection_asks.borrow_mut().push((kind, direction));
        self.connect
            .get(&(kind, neighbour))
            .copied()
            .unwrap_or(vanilla)
    }
    fn adjust_analog_output(
        &self,
        pos: BlockPos,
        _state: BlockStateId,
        _direction: Direction,
        vanilla: i32,
    ) -> i32 {
        self.analog.get(&pos).copied().unwrap_or(vanilla)
    }
    fn adjust_signal(
        &self,
        pos: BlockPos,
        _state: BlockStateId,
        direction: Direction,
        direct: bool,
        vanilla: i32,
    ) -> i32 {
        match self.signals.get(&pos) {
            Some((_, _, Some(only))) if *only != direction => 0,
            Some((weak, strong, _)) => {
                if direct {
                    *strong
                } else {
                    *weak
                }
            }
            None => vanilla,
        }
    }
    fn is_client_side(&self) -> bool {
        self.client_side
    }
    fn block_state(&self, pos: BlockPos) -> BlockStateId {
        self.blocks.get(&pos).copied().unwrap_or(self.air)
    }
    fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
        Some(self.blocks.insert(pos, state).unwrap_or(self.air))
    }
    fn is_loaded(&self, _pos: BlockPos) -> bool {
        true
    }
    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }
    fn game_time(&self) -> i64 {
        self.time
    }
    fn day_time(&self) -> i64 {
        self.day_time
    }
    fn block_light(&self, pos: BlockPos) -> i32 {
        self.block_light.get(&pos).copied().unwrap_or(0)
    }
    fn emit(&mut self, event: LevelEvent) {
        self.events.push(event);
    }
    fn allow_spread(&self, _kind: Spread, _from: BlockPos, to: BlockPos) -> bool {
        !self.guarded.contains(&to)
    }
    fn allow_piston_move(
        &self,
        _piston: BlockPos,
        direction: Direction,
        _extending: bool,
        push: &[BlockPos],
        destroy: &[BlockPos],
    ) -> bool {
        !push
            .iter()
            .flat_map(|&p| [p, update::rel(p, direction)])
            .chain(destroy.iter().copied())
            .any(|p| self.guarded.contains(&p))
    }
    fn before_fire_odds(&self, state: BlockStateId, odds: &mut Option<(i32, i32)>) {
        if let Some(&answer) = self.fire_odds.get(&state) {
            *odds = Some(answer);
        }
    }
    fn before_smoke_source(&self, state: BlockStateId, smoke: &mut Option<bool>) {
        if let Some(&answer) = self.smoke_sources.get(&state) {
            *smoke = Some(answer);
        }
    }
    fn note_block_play(
        &self,
        _pos: BlockPos,
        by_player: bool,
        instrument: &mut u8,
        note: &mut i32,
    ) -> bool {
        self.note_players.borrow_mut().push(by_player);
        match self.note_answer {
            None => true,
            Some(None) => false,
            Some(Some((i, n))) => {
                (*instrument, *note) = (i, n);
                true
            }
        }
    }
    fn allow_portal_spawn(
        &self,
        _pos: BlockPos,
        _state: BlockStateId,
        _shape: &PortalShape,
    ) -> bool {
        !self.refuse_portals
    }
}

struct World<'f> {
    f: &'f Fixture,
    level: TestLevel,
    ticks: Ticks,
}
impl<'f> World<'f> {
    fn new(f: &'f Fixture) -> Self {
        let mut ticks = Ticks::new(0);
        for x in -4..4 {
            for z in -4..4 {
                ticks.add_empty_chunk(ChunkPos::new(x, z)).unwrap();
            }
        }
        Self {
            level: TestLevel::new(&f.behaviours),
            f,
            ticks,
        }
    }
    fn b(&self) -> &'f Behaviours {
        &self.f.behaviours
    }
    fn id(&self, name: &str, props: &[(&str, &str)]) -> BlockStateId {
        let state = BlockState {
            name: format!("minecraft:{name}"),
            properties: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        self.f
            .registry
            .resolve(&state)
            .unwrap_or_else(|_| panic!("unknown state {name} {props:?}"))
    }
    fn set(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        self.f.behaviours.set_block(
            &mut self.level,
            &mut self.ticks,
            pos,
            state,
            flags::UPDATE_ALL,
        )
    }
    /// Raw storage write without updates (world generation).
    fn put(&mut self, pos: BlockPos, state: BlockStateId) {
        self.level.blocks.insert(pos, state);
    }
    fn get(&self, pos: BlockPos) -> BlockStateId {
        self.level.block_state(pos)
    }
    fn name(&self, pos: BlockPos) -> String {
        let state = self.f.registry.state(self.get(pos)).unwrap();
        let props: Vec<String> = state
            .properties
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        format!(
            "{}[{}]",
            state.name.trim_start_matches("minecraft:"),
            props.join(",")
        )
    }
    fn prop(&self, pos: BlockPos, key: &str) -> String {
        self.f
            .registry
            .state(self.get(pos))
            .unwrap()
            .properties
            .get(key)
            .cloned()
            .unwrap_or_default()
    }
    fn tick(&mut self) {
        self.level.time += 1;
        self.f
            .behaviours
            .tick_scheduled(&mut self.level, &mut self.ticks);
    }
    /// Scheduled ticks, block events and block entities, as `ServerLevel.tick`.
    fn full_tick(&mut self) {
        self.tick();
        self.f
            .behaviours
            .run_block_events(&mut self.level, &mut self.ticks);
        self.f
            .behaviours
            .tick_block_entities(&mut self.level, &mut self.ticks);
    }
    fn floor(&mut self, y: i32, radius: i32, block: &str) {
        let state = self.id(block, &[]);
        for x in -radius..=radius {
            for z in -radius..=radius {
                self.put(BlockPos::new(x, y, z), state);
            }
        }
    }
}

macro_rules! world {
    () => {{
        let Some(f) = fixture() else {
            return;
        };
        World::new(f)
    }};
}

#[test]
fn block_observation_follows_successful_commits_only() {
    use std::cell::RefCell;

    #[derive(Debug, PartialEq)]
    enum Observation {
        Block(BlockPos, BlockStateId, BlockStateId),
        Neighbor,
    }
    struct Observer {
        inner: TestLevel,
        loaded: bool,
        seen: RefCell<Vec<Observation>>,
    }
    impl LevelAccess for Observer {
        fn block_state(&self, pos: BlockPos) -> BlockStateId {
            self.inner.block_state(pos)
        }
        fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
            self.inner.set_raw(pos, state)
        }
        fn is_loaded(&self, _pos: BlockPos) -> bool {
            self.loaded
        }
        fn random(&mut self) -> &mut LegacyRandom {
            self.inner.random()
        }
        fn game_time(&self) -> i64 {
            self.inner.game_time()
        }
        fn observe_block_changed(&self, pos: BlockPos, old: BlockStateId, new: BlockStateId) {
            assert_eq!(self.block_state(pos), new);
            self.seen
                .borrow_mut()
                .push(Observation::Block(pos, old, new));
        }
        fn observe_neighbor_changed(
            &self,
            _: &mut Ticks,
            _: BlockPos,
            _: BlockStateId,
            _: BlockId,
            _: bool,
        ) {
            self.seen.borrow_mut().push(Observation::Neighbor);
        }
    }
    let Some(f) = fixture() else { return };
    let b = &f.behaviours;
    let mut level = Observer {
        inner: TestLevel::new(b),
        loaded: true,
        seen: RefCell::new(Vec::new()),
    };
    let mut ticks = Ticks::new(0);
    let pos = BlockPos::new(0, 80, 0);
    let stone = f.registry.default_state("minecraft:stone").unwrap();
    assert!(b.set_block(&mut level, &mut ticks, pos, stone, flags::UPDATE_ALL));
    let seen = level.seen.borrow();
    assert_eq!(seen[0], Observation::Block(pos, b.air(), stone));
    assert!(seen[1..].contains(&Observation::Neighbor));
    drop(seen);
    level.seen.borrow_mut().clear();
    assert!(!b.set_block(&mut level, &mut ticks, pos, stone, flags::UPDATE_ALL));
    level.loaded = false;
    assert!(!b.set_block(&mut level, &mut ticks, pos, b.air(), flags::UPDATE_ALL));
    level.loaded = true;
    assert!(!b.set_block(
        &mut level,
        &mut ticks,
        BlockPos::new(0, 320, 0),
        stone,
        flags::UPDATE_ALL
    ));
    assert!(level.seen.borrow().is_empty());
    assert!(b.set_block(&mut level, &mut ticks, pos, b.air(), flags::UPDATE_ALL));
    assert_eq!(
        level.seen.borrow()[0],
        Observation::Block(pos, stone, b.air())
    );
}

#[test]
fn neighbor_observation_follows_dispatch_order() {
    use std::cell::RefCell;

    struct Observer {
        inner: TestLevel,
        seen: RefCell<Vec<(BlockPos, BlockStateId, BlockId, bool)>>,
    }
    impl LevelAccess for Observer {
        fn block_state(&self, pos: BlockPos) -> BlockStateId {
            self.inner.block_state(pos)
        }
        fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
            self.inner.set_raw(pos, state)
        }
        fn is_loaded(&self, pos: BlockPos) -> bool {
            self.inner.is_loaded(pos)
        }
        fn random(&mut self) -> &mut LegacyRandom {
            self.inner.random()
        }
        fn game_time(&self) -> i64 {
            self.inner.game_time()
        }
        fn observe_neighbor_changed(
            &self,
            _: &mut Ticks,
            pos: BlockPos,
            state: BlockStateId,
            changed: BlockId,
            moved: bool,
        ) {
            assert_eq!(self.block_state(pos), state);
            self.seen.borrow_mut().push((pos, state, changed, moved));
        }
    }
    let Some(f) = fixture() else { return };
    let b = &f.behaviours;
    let mut level = Observer {
        inner: TestLevel::new(b),
        seen: RefCell::new(Vec::new()),
    };
    let mut ticks = Ticks::new(0);
    let changed = b.table.block_by_name("minecraft:stone").unwrap();
    b.update_neighbors_at(&mut level, &mut ticks, BlockPos::new(0, 0, 0), changed);
    let expected = [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
    ];
    let actual = level.seen.borrow();
    assert_eq!(actual.len(), expected.len());
    for (event, (x, y, z)) in actual.iter().zip(expected) {
        assert_eq!(*event, (BlockPos::new(x, y, z), b.air(), changed, false));
    }
    drop(actual);
    b.sim(&mut level, &mut ticks)
        .neighbor_changed(b.air(), BlockPos::new(3, 4, 5), changed, true);
    assert_eq!(
        level.seen.borrow().last(),
        Some(&(BlockPos::new(3, 4, 5), b.air(), changed, true))
    );
}

    }

/// `WeatheringCopperChestBlock.randomTick`: a copper chest someone has open
/// skips `changeOverTime` and its draws; a closed one draws (#11378).
#[test]
fn a_viewed_copper_chest_does_not_weather() {
    let Some(f) = fixture() else { return };
    let mut w = World::new(f);
    let pos = BlockPos::new(0, 64, 0);
    let chest = w.id("copper_chest", &[("type", "single")]);
    w.put(pos, chest);
    let draw = |w: &World| w.level.random.clone().next_long();
    w.level.opened.insert(pos);
    let before = draw(&w);
    f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    assert_eq!(draw(&w), before, "an open chest draws nothing");
    w.level.opened.clear();
    f.behaviours.random_tick(&mut w.level, &mut w.ticks, pos);
    assert_ne!(draw(&w), before, "a closed one rolls changeOverTime");
