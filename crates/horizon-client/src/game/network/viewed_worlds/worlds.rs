//! The worlds of the levels a client views: one `World` per level the server opened, with the
//! chunk format, shape and facts of the dimension type the server synced for it. A wrapped
//! packet changes only its own level's world; nothing here reaches the player's level, its
//! renderer, its sounds or a mod's hooks.
use super::{entities::ViewedEntities, routing::RouteError};
use crate::game::remote_chunks::{ChunkFormat, section_blocks};
use horizon_client::{chunk::known_block_state, fast_hash::FxHashMap, world::World};
use horizon_protocol::packets::play::{Clientbound as C, ClockState};
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

/// The most columns one viewed world holds: the server's own cap on a player's `Loaded` chunks in
/// one level (`max_viewed_loaded_chunks`, 2048 by default), so that a server cannot fill the
/// client's memory through one opened level. A server that raises its setting past it is refused.
pub(in crate::game) const MAX_COLUMNS: usize = 2048;

/// Another level's world, kept while the server sends it.
pub(in crate::game) struct ViewedWorld {
    /// The level's key (`minecraft:the_nether`).
    pub(in crate::game) level: String,
    /// What its chunk packets decode with.
    format: Arc<ChunkFormat>,
    /// The world clock its day time follows: its dimension type's `default_clock`.
    default_clock: Option<i32>,
    /// That clock as the server last told it, moved on with the game time since
    /// (`ClientClockManager`).
    clock: Option<ClockState>,
    pub(in crate::game) world: World,
    pub(in crate::game) entities: ViewedEntities,
}

impl ViewedWorld {
    pub(in crate::game) fn new(
        level: String,
        format: Arc<ChunkFormat>,
        default_clock: Option<i32>,
        world: World,
    ) -> Self {
        Self {
            level,
            format,
            default_clock,
            clock: None,
            world,
            entities: ViewedEntities::default(),
        }
    }

    /// A wrapped packet of the level, as `ClientPacketListener` handles it with that level
    /// current: what the world keeps of its chunks, light, biomes, blocks, time, weather and
    /// entities. Block events, destruction progress and level events change nothing yet: what
    /// they show is drawn, and drawing a viewed level is gap 13. A chunk of a new column past
    /// [`MAX_COLUMNS`] is refused before it is decoded.
    pub(in crate::game) fn apply(&mut self, packet: C) -> Result<(), RouteError> {
        match packet {
            // `handleLevelChunkWithLight`: the column replaces any it held, with the light the
            // server sent.
            C::LevelChunkWithLight(p)
                if self.world.chunks.len() >= MAX_COLUMNS
                    && !self.world.chunks.contains_key(&(p.x, p.z)) =>
            {
                return Err(RouteError::TooManyColumns);
            }
            C::LevelChunkWithLight(p) => match self.format.column(&p) {
                Ok(chunk) => {
                    self.world.chunks.insert((p.x, p.z), Arc::new(chunk));
                }
                Err(e) => log::warn!("Horizon: {} chunk {} {}: {e}", self.level, p.x, p.z),
            },
            C::ForgetLevelChunk(p) => {
                self.world.unload_chunk((p.pos[0], p.pos[1]));
            }
            C::LightUpdate(p) => {
                let (layers, _) = self.format.light(&p.light);
                if let Some(chunk) = self.world.chunks.get_mut(&(p.x, p.z)) {
                    horizon_client::light::update_light_layers(
                        Arc::make_mut(chunk),
                        &layers,
                        self.format.has_skylight,
                    );
                }
            }
            C::ChunksBiomes(p) => {
                for data in &p.chunks {
                    let [x, z] = data.pos;
                    if let Some(chunk) = self.world.chunks.get_mut(&(x, z))
                        && let Err(e) = self.format.replace_biomes(Arc::make_mut(chunk), data)
                    {
                        log::warn!("Horizon: {} biomes of chunk {x} {z}: {e}", self.level);
                    }
                }
            }
            C::BlockUpdate(p) => self.set_block(p.pos, p.state),
            C::SectionBlocksUpdate(p) => {
                for (pos, state) in section_blocks(&p) {
                    self.set_block(pos, state);
                }
            }
            // `handleSetTime` / `ClientClockManager.handleUpdates`: the clock it follows moves on
            // by the game time since the last packet at its rate, then takes what the packet
            // carries of it; its day follows that clock.
            C::SetTime(p) => {
                let delta = p.game_time - self.world.game_time;
                self.world.game_time = p.game_time;
                if let Some(clock) = &mut self.clock {
                    crate::game::remote::client_level_tick::advance_clock(clock, delta);
                }
                if let Some(update) = p
                    .clocks
                    .into_iter()
                    .find(|c| Some(c.clock) == self.default_clock)
                {
                    self.clock = Some(update);
                }
                if let Some(clock) = &self.clock {
                    self.world.day_time = clock.total_ticks;
                }
            }
            C::GameEvent(p) => self.world.weather.game_event(p.event, p.param),
            other => self.entities.apply(other)?,
        }
        Ok(())
    }

    /// A server's block: a state the client lacks (a server with mods it lacks) reads as air,
    /// as in its chunks.
    fn set_block(&mut self, [x, y, z]: [i32; 3], state: i32) {
        let state = state.max(0) as u32;
        let state = BlockStateId(if known_block_state(state) { state } else { 0 });
        self.world.set_stored(x, y, z, state);
    }
}

/// The worlds a connection views (`Remote::viewed`), by the level's index in the Login packet's
/// `levels`.
#[derive(Default)]
pub(in crate::game) struct ViewedWorlds {
    worlds: FxHashMap<i32, ViewedWorld>,
}

impl ViewedWorlds {
    /// How many levels it views.
    pub(in crate::game) fn len(&self) -> usize {
        self.worlds.len()
    }
    pub(in crate::game) fn contains(&self, level: i32) -> bool {
        self.worlds.contains_key(&level)
    }
    pub(in crate::game) fn get_mut(&mut self, level: i32) -> Option<&mut ViewedWorld> {
        self.worlds.get_mut(&level)
    }
    /// The world of `level`, if the server opened it.
    #[cfg(test)]
    pub(in crate::game) fn get(&self, level: i32) -> Option<&ViewedWorld> {
        self.worlds.get(&level)
    }
    /// `level`'s world, replacing any it held: an `open` starts afresh.
    pub(in crate::game) fn insert(&mut self, level: i32, world: ViewedWorld) {
        if let Some(old) = self.worlds.insert(level, world) {
            free(old);
        }
    }
    /// Drops `level`'s world (`close`); false when it held none.
    pub(in crate::game) fn remove(&mut self, level: i32) -> bool {
        self.worlds.remove(&level).map(free).is_some()
    }
    /// Drops every world: a Login, a Respawn or a new configuration phase.
    pub(in crate::game) fn clear(&mut self) {
        for (_, world) in self.worlds.drain() {
            free(world);
        }
    }
}

/// A world given up: its columns are freed off the game thread when there are many.
fn free(mut world: ViewedWorld) {
    let chunks = std::mem::take(&mut world.world.chunks);
    let count = chunks.len();
    horizon_client::chunk::free_columns(chunks, count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_protocol::{
        chunk::{ChunkData, LightData, Paletted, Section, Strategy},
        packets::play,
        registry::DimensionType,
    };

    /// A column of stone at `x, 0`, unlit.
    fn column(x: i32) -> C {
        let overworld = DimensionType::vanilla("minecraft:overworld");
        let section = Section {
            non_empty: 4096,
            fluids: 0,
            states: Paletted::from_values(&[1; 4096], Strategy::Blocks, 16),
            biomes: Paletted::from_values(&[1; 64], Strategy::Biomes, 2),
        };
        C::LevelChunkWithLight(play::LevelChunkWithLight {
            x,
            z: 0,
            chunk: ChunkData {
                heightmaps: Vec::new(),
                buffer: ChunkData::from_sections(&vec![section; overworld.sections()]),
                block_entities: Vec::new(),
            },
            light: LightData {
                sky_mask: Vec::new(),
                block_mask: Vec::new(),
                empty_sky_mask: Vec::new(),
                empty_block_mask: Vec::new(),
                sky: Vec::new(),
                block: Vec::new(),
            },
        })
    }

    /// A world full at 2048 columns takes a column it holds again, refuses a new one, and takes
    /// it once the server forgot another.
    #[test]
    fn a_viewed_world_holds_at_most_2048_columns() {
        let overworld = DimensionType::vanilla("minecraft:overworld");
        let format = Arc::new(ChunkFormat::new(&overworld, vec![0, 7, 9], 2));
        let mut viewed = ViewedWorld::new(
            "minecraft:the_nether".into(),
            format,
            None,
            World::new(BlockStateId(0), 1),
        );
        for x in 0..2048 {
            viewed.apply(column(x)).unwrap();
        }
        assert_eq!(viewed.world.chunks.len(), 2048);
        // A replacement is no new column.
        assert_eq!(viewed.apply(column(17)), Ok(()));
        assert_eq!(viewed.world.chunks.len(), 2048);
        assert_eq!(viewed.apply(column(2048)), Err(RouteError::TooManyColumns));
        assert!(!viewed.world.chunks.contains_key(&(2048, 0)));
        let forget = C::ForgetLevelChunk(play::ForgetLevelChunk { pos: [5, 0] });
        viewed.apply(forget).unwrap();
        assert_eq!(viewed.apply(column(2048)), Ok(()));
        assert!(viewed.world.chunks.contains_key(&(2048, 0)));
        assert_eq!(viewed.world.chunks.len(), 2048);
    }

    /// A time update with no clocks moves the retained clock on by the game time since the last
    /// update at its rate, as the player's own level's does: 1000 at time 100, then an empty
    /// update at time 120, is 1020. A clock at half rate carries its half tick.
    #[test]
    fn an_empty_time_update_moves_the_retained_clock_on() {
        let overworld = DimensionType::vanilla("minecraft:overworld");
        let format = Arc::new(ChunkFormat::new(&overworld, vec![0, 7, 9], 2));
        let mut viewed = ViewedWorld::new(
            "minecraft:the_end".into(),
            format,
            Some(3),
            World::new(BlockStateId(0), 1),
        );
        let time = |game_time, clocks| C::SetTime(play::SetTime { game_time, clocks });
        let clock = |clock, total_ticks, rate| ClockState {
            clock,
            total_ticks,
            partial_tick: 0.0,
            rate,
        };
        viewed
            .apply(time(100, vec![clock(0, 7, 1.0), clock(3, 1000, 1.0)]))
            .unwrap();
        assert_eq!(viewed.world.day_time, 1000);
        viewed.apply(time(120, Vec::new())).unwrap();
        assert_eq!(viewed.world.day_time, 1020);
        assert_eq!(viewed.world.game_time, 120);
        viewed.apply(time(120, vec![clock(3, 2000, 0.5)])).unwrap();
        viewed.apply(time(123, Vec::new())).unwrap();
        assert_eq!(viewed.world.day_time, 2001);
        viewed.apply(time(124, Vec::new())).unwrap();
        assert_eq!(viewed.world.day_time, 2002);
    }
}
