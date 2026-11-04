//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
use std::sync::Mutex;
/// What a ticket belongs to: a block (a machine) or an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TicketOwner {
    /// The block at `[x, y, z]`.
    Block([i32; 3]),
    /// The entity with this UUID.
    Entity(u128),
}
/// One chunk a mod holds loaded for an owner.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModTicket {
    /// The controller that holds it: its slot in the installed controller table
    /// (`mod_api::TicketController::slot`).
    pub controller: u32,
    pub owner: TicketOwner,
    pub chunk: (i32, i32),
    /// Whether the chunk joins the spawning chunks (inhabited time, thunder,
    /// a spawn attempt) with no player within 128 blocks
    /// (`forceChunk`'s `forceNaturalSpawning`). One owner holds the two kinds
    /// apart.
    pub natural_spawning: bool,
}
/// The chunks one owner holds, the two kinds apart (NeoForge's `TicketSet`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TicketSet {
    /// Chunks held without natural spawning.
    pub non_natural_spawning: Vec<[i32; 2]>,
    /// Chunks held with natural spawning forced.
    pub natural_spawning: Vec<[i32; 2]>,
}
impl TicketSet {
    fn push(&mut self, chunk: [i32; 2], natural_spawning: bool) {
        if natural_spawning {
            self.natural_spawning.push(chunk);
        } else {
            self.non_natural_spawning.push(chunk);
        }
    }
}
/// The tickets one controller loaded, as `validate_tickets` hands them out
/// (NeoForge's `TicketHelper`, one for each controller). Every call acts as
/// the mod marked as running, and only the controller's owning mod sees or
/// removes anything, block owners and entity owners apart. The removals made
/// are the game's: a removal goes to a [`TicketRemovals`] queue only the game
/// holds.
pub struct TicketValidation {
    /// The `[mod] id` that owns the controller.
    owner: &'static str,
    loaded: Vec<ModTicket>,
    removed: Arc<Mutex<Vec<ModTicket>>>,
}
impl std::fmt::Debug for TicketValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TicketValidation")
            .field("own", &self.own().count())
            .finish()
    }
}
/// The tickets the mods removed in one validation, for the game to release.
#[derive(Debug)]
pub struct TicketRemovals(Arc<Mutex<Vec<ModTicket>>>);
impl TicketRemovals {
    /// The tickets removed.
    pub fn into_removed(self) -> Vec<ModTicket> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}
impl TicketValidation {
    /// A validation of the `loaded` tickets of one controller, owned by mod
    /// `owner`, and the queue of what is removed. For the game.
    #[doc(hidden)]
    pub fn new(owner: &'static str, loaded: Vec<ModTicket>) -> (Self, TicketRemovals) {
        let removed = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                owner,
                loaded,
                removed: removed.clone(),
            },
            TicketRemovals(removed),
        )
    }
    /// The controller's block tickets: each block position with the chunks it
    /// holds without and with natural spawning (`TicketHelper.getBlockTickets`'
    /// `TicketSet`: `nonNaturalSpawning`, `naturalSpawning`), in position order.
    pub fn block_tickets(&self) -> Vec<([i32; 3], TicketSet)> {
        let mut grouped = std::collections::BTreeMap::<[i32; 3], TicketSet>::new();
        for (owner, chunk, spawning) in self.own() {
            if let TicketOwner::Block(pos) = owner {
                grouped.entry(pos).or_default().push(chunk, spawning);
            }
        }
        grouped.into_iter().collect()
    }
    /// The controller's entity tickets: each entity UUID with the chunks it
    /// holds without and with natural spawning (`TicketHelper.getEntityTickets`),
    /// in UUID order.
    pub fn entity_tickets(&self) -> Vec<(u128, TicketSet)> {
        let mut grouped = std::collections::BTreeMap::<u128, TicketSet>::new();
        for (owner, chunk, spawning) in self.own() {
            if let TicketOwner::Entity(uuid) = owner {
                grouped.entry(uuid).or_default().push(chunk, spawning);
            }
        }
        grouped.into_iter().collect()
    }
    /// Remove every ticket the block holds, of both kinds
    /// (`removeAllTickets(BlockPos)`): how many went.
    pub fn remove_block_tickets(&mut self, pos: [i32; 3]) -> usize {
        self.remove(TicketOwner::Block(pos), None)
    }
    /// Remove every ticket the entity holds, of both kinds
    /// (`removeAllTickets(UUID)`).
    pub fn remove_entity_tickets(&mut self, uuid: u128) -> usize {
        self.remove(TicketOwner::Entity(uuid), None)
    }
    /// Remove the block's ticket of one kind on one chunk
    /// (`removeTicket(BlockPos, ChunkPos, forceNaturalSpawning)`): whether it
    /// held one. The other kind on the chunk stays.
    pub fn remove_block_ticket(
        &mut self,
        pos: [i32; 3],
        chunk: [i32; 2],
        natural_spawning: bool,
    ) -> bool {
        self.remove(TicketOwner::Block(pos), Some((chunk, natural_spawning))) > 0
    }
    /// Remove the entity's ticket of one kind on one chunk
    /// (`removeTicket(UUID, ChunkPos, forceNaturalSpawning)`).
    pub fn remove_entity_ticket(
        &mut self,
        uuid: u128,
        chunk: [i32; 2],
        natural_spawning: bool,
    ) -> bool {
        self.remove(TicketOwner::Entity(uuid), Some((chunk, natural_spawning))) > 0
    }
    fn own(&self) -> impl Iterator<Item = (TicketOwner, [i32; 2], bool)> + '_ {
        let owned = crate::crash::current_native_mod() == Some(self.owner);
        self.loaded
            .iter()
            .filter(move |_| owned)
            .map(|t| (t.owner, [t.chunk.0, t.chunk.1], t.natural_spawning))
    }
    fn remove(&mut self, owner: TicketOwner, kind: Option<([i32; 2], bool)>) -> usize {
        if crate::crash::current_native_mod() != Some(self.owner) {
            return 0;
        }
        let (gone, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.loaded).into_iter().partition(|t| {
                t.owner == owner
                    && kind.is_none_or(|([x, z], spawning)| {
                        t.chunk == (x, z) && t.natural_spawning == spawning
                    })
            });
        self.loaded = kept;
        let count = gone.len();
        self.removed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(gone);
        count
    }
}
