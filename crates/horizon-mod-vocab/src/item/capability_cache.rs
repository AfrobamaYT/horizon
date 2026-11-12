//! `BlockCapabilityCache`: a block capability looked up once and kept, for a pipe that asks its
//! neighbours every tick (NeoForge's `BlockCapabilityCache`, without its listener: this cache is
//! polled). Its check is constant time: the block entity at the position, the block state there
//! and the count of mod switches, each compared with what they were at the lookup.
use crate::{BlockEntityRef, BlockSide, BlockState, Capability, CapabilityKey, Direction, LevelRef, ServerHost};

/// A block capability looked up once and kept. [`get`](Self::get) gives the kept handler while
/// the block entity at its position is the same one (not replaced, removed or unloaded with its
/// chunk), the block state there is the same (a chest turned is asked again), and no mod has
/// been switched on or off; otherwise it looks the capability up again. A level that closes
/// leaves its handles stale, and the cache then finds nothing. A provider whose answer changes
/// with none of that (a machine's side configuration) is told with
/// [`invalidate`](Self::invalidate).
// no wire: holds an owned handler a lookup made in this process; a mod keeps it, never sends it.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockCapabilityCache<T: ?Sized + 'static> {
    key: &'static CapabilityKey<T>,
    at: BlockSide,
    kept: Option<Kept<T>>,
}

/// What a lookup found, and what it was valid for.
struct Kept<T: ?Sized> {
    entity: Option<BlockEntityRef>,
    state: Option<BlockState>,
    mods: u64,
    handler: Option<Capability<T>>,
}

impl<T: ?Sized + 'static> BlockCapabilityCache<T> {
    /// A cache of `key` at `pos` in `level`, reached through `side`; nothing is looked up yet.
    pub fn new(
        key: &'static CapabilityKey<T>,
        level: LevelRef,
        pos: [i32; 3],
        side: Option<Direction>,
    ) -> Self {
        Self {
            key,
            at: BlockSide { level, pos, side },
            kept: None,
        }
    }
    /// The handler there, if any: the kept one while what it was valid for still holds, else
    /// looked up again ([`LevelRef::capability`]).
    pub fn get(&mut self, host: &mut dyn ServerHost) -> Option<&Capability<T>> {
        let entity = host.block_entity(self.at.level, self.at.pos);
        let state = self.at.level.block_state(&*host, self.at.pos);
        let mods = crate::runtime::mods_switched();
        let valid = |kept: &Kept<T>| kept.entity == entity && kept.state == state && kept.mods == mods;
        if !self.kept.as_ref().is_some_and(valid) {
            let handler = self
                .at
                .level
                .capability(host, self.key, self.at.pos, self.at.side);
            self.kept = Some(Kept {
                entity,
                state,
                mods,
                handler,
            });
        }
        self.kept.as_ref()?.handler.as_ref()
    }
    /// Forgets the kept handler, so the next [`get`](Self::get) looks it up again.
    pub fn invalidate(&mut self) {
        self.kept = None;
    }
    /// Where it looks.
    pub fn at(&self) -> BlockSide {
        self.at
    }
}
