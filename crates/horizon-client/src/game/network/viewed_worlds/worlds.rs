//! The worlds of the levels a client views: one `World` per level the server opened, with the
//! chunk format, shape and facts of the dimension type the server synced for it. A wrapped
//! packet changes only its own level's world; nothing here reaches the player's level, its
//! renderer, its sounds or a mod's hooks.
use crate::game::remote_chunks::{ChunkFormat, section_blocks};
use horizon_client::{chunk::known_block_state, fast_hash::FxHashMap, world::World};
use horizon_protocol::packets::play::Clientbound as C;
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

/// Another level's world, kept while the server sends it.
pub(in crate::game) struct ViewedWorld {
    /// The level's key (`minecraft:the_nether`).
    pub(in crate::game) level: String,
    /// What its chunk packets decode with.
    format: Arc<ChunkFormat>,
    /// The world clock its day time follows: its dimension type's `default_clock`.
    default_clock: Option<i32>,
    pub(in crate::game) world: World,
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
            world,
        }
    }

    /// A wrapped packet of the level, as `ClientPacketListener` handles it with that level
    /// current: what the world keeps of its chunks, light, biomes, blocks, time and weather.
    /// Block events, destruction progress, level events and entities change nothing yet: what
    /// they show is drawn, and drawing a viewed level is gap 13 (entities are 9d).
    pub(in crate::game) fn apply(&mut self, packet: C) {
        match packet {
            // `handleLevelChunkWithLight`: the column replaces any it held, with the light the
            // server sent.
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
            // `handleSetTime`: the level's game time, and its day from the clock it follows
            // when the packet carries that clock.
            C::SetTime(p) => {
                self.world.game_time = p.game_time;
                if let Some(clock) = p
                    .clocks
                    .iter()
                    .find(|c| Some(c.clock) == self.default_clock)
                {
                    self.world.day_time = clock.total_ticks;
                }
            }
            C::GameEvent(p) => self.world.weather.game_event(p.event, p.param),
            _ => {}
        }
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
