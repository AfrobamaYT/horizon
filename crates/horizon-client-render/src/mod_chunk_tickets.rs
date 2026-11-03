//! The chunk tickets native mods hold (NeoForge's `TicketController`): one
//! ticket per (mod, owner, chunk), so two machines in one chunk don't release
//! each other's. `Streaming` keeps the union with `/forceload`'s chunks in its
//! `forced` set, which is all loading and ticking read.
use std::collections::{BTreeSet, HashMap};

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
    /// The holding mod's `[mod] id`.
    pub mod_id: String,
    pub owner: TicketOwner,
    pub chunk: (i32, i32),
}

/// The tickets of one level.
#[derive(Debug, Default)]
pub struct ModTickets {
    held: BTreeSet<ModTicket>,
    per_chunk: HashMap<(i32, i32), u32>,
}

impl ModTickets {
    /// Tickets read from a save: those of a mod `loaded` accepts are held, the
    /// others are discarded (`TicketController`: a controller that isn't
    /// registered loses its tickets when the world loads).
    pub fn from_saved(saved: Vec<ModTicket>, loaded: impl Fn(&str) -> bool) -> Self {
        let mut tickets = Self::default();
        for ticket in saved {
            if loaded(&ticket.mod_id) {
                tickets.insert(ticket);
            }
        }
        tickets
    }
    /// Hold `ticket`: true when it was new.
    pub fn insert(&mut self, ticket: ModTicket) -> bool {
        let chunk = ticket.chunk;
        let new = self.held.insert(ticket);
        if new {
            *self.per_chunk.entry(chunk).or_default() += 1;
        }
        new
    }
    /// Release `ticket`: true when it was held.
    pub fn remove(&mut self, ticket: &ModTicket) -> bool {
        let held = self.held.remove(ticket);
        if held && let Some(count) = self.per_chunk.get_mut(&ticket.chunk) {
            *count -= 1;
            if *count == 0 {
                self.per_chunk.remove(&ticket.chunk);
            }
        }
        held
    }
    /// Whether any mod holds `chunk`.
    pub fn holds(&self, chunk: (i32, i32)) -> bool {
        self.per_chunk.contains_key(&chunk)
    }
    /// The chunks any mod holds.
    pub fn chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.per_chunk.keys().copied()
    }
    /// Whether no mod holds a ticket.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
    /// The tickets mod `mod_id` holds, in order.
    pub fn of_mod<'a>(&'a self, mod_id: &'a str) -> impl Iterator<Item = &'a ModTicket> {
        self.held.iter().filter(move |t| t.mod_id == mod_id)
    }
    /// Every ticket to save.
    pub fn to_save(&self) -> impl Iterator<Item = &ModTicket> {
        self.held.iter()
    }
}

/// The tickets a level loaded, as `chunk_tickets_validate` hands them out
/// (NeoForge's `TicketHelper`): each mod sees and removes only its own,
/// block owners and entity owners apart.
#[derive(Debug, Default)]
pub struct TicketValidation {
    loaded: Vec<ModTicket>,
    removed: Vec<ModTicket>,
}

impl TicketValidation {
    /// The tickets of a level, all mods'.
    pub fn new(loaded: Vec<ModTicket>) -> Self {
        Self {
            loaded,
            removed: Vec::new(),
        }
    }
    /// The tickets removed so far, to release.
    pub fn into_removed(self) -> Vec<ModTicket> {
        self.removed
    }
    /// The calling mod's block tickets: each block position and the chunks it
    /// holds (`TicketHelper.getBlockTickets`), in position order.
    pub fn block_tickets(&self) -> Vec<([i32; 3], Vec<[i32; 2]>)> {
        let mut grouped = std::collections::BTreeMap::<[i32; 3], Vec<[i32; 2]>>::new();
        for (owner, chunk) in self.own() {
            if let TicketOwner::Block(pos) = owner {
                grouped.entry(pos).or_default().push(chunk);
            }
        }
        grouped.into_iter().collect()
    }
    /// The calling mod's entity tickets: each entity UUID and the chunks it
    /// holds (`TicketHelper.getEntityTickets`), in UUID order.
    pub fn entity_tickets(&self) -> Vec<(u128, Vec<[i32; 2]>)> {
        let mut grouped = std::collections::BTreeMap::<u128, Vec<[i32; 2]>>::new();
        for (owner, chunk) in self.own() {
            if let TicketOwner::Entity(uuid) = owner {
                grouped.entry(uuid).or_default().push(chunk);
            }
        }
        grouped.into_iter().collect()
    }
    /// Remove every ticket the block holds (`removeAllTickets(BlockPos)`):
    /// how many went.
    pub fn remove_block_tickets(&mut self, pos: [i32; 3]) -> usize {
        self.remove(TicketOwner::Block(pos), None)
    }
    /// Remove every ticket the entity holds (`removeAllTickets(UUID)`).
    pub fn remove_entity_tickets(&mut self, uuid: u128) -> usize {
        self.remove(TicketOwner::Entity(uuid), None)
    }
    /// Remove the block's ticket on one chunk (`removeTicket(BlockPos, ...)`):
    /// whether it held one.
    pub fn remove_block_ticket(&mut self, pos: [i32; 3], chunk: [i32; 2]) -> bool {
        self.remove(TicketOwner::Block(pos), Some(chunk)) > 0
    }
    /// Remove the entity's ticket on one chunk (`removeTicket(UUID, ...)`).
    pub fn remove_entity_ticket(&mut self, uuid: u128, chunk: [i32; 2]) -> bool {
        self.remove(TicketOwner::Entity(uuid), Some(chunk)) > 0
    }
    fn own(&self) -> impl Iterator<Item = (TicketOwner, [i32; 2])> + '_ {
        let me = crate::crash::current_native_mod();
        self.loaded
            .iter()
            .filter(move |t| me == Some(t.mod_id.as_str()))
            .map(|t| (t.owner, [t.chunk.0, t.chunk.1]))
    }
    fn remove(&mut self, owner: TicketOwner, chunk: Option<[i32; 2]>) -> usize {
        let Some(me) = crate::crash::current_native_mod() else {
            return 0;
        };
        let (gone, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.loaded).into_iter().partition(|t| {
                t.mod_id == me && t.owner == owner && chunk.is_none_or(|[x, z]| t.chunk == (x, z))
            });
        self.loaded = kept;
        let count = gone.len();
        self.removed.extend(gone);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(mod_id: &str, owner: TicketOwner, chunk: (i32, i32)) -> ModTicket {
        ModTicket {
            mod_id: mod_id.into(),
            owner,
            chunk,
        }
    }

    #[test]
    fn a_chunk_is_held_while_any_ticket_holds_it() {
        let mut tickets = ModTickets::default();
        let (a, b) = (TicketOwner::Block([1, 2, 3]), TicketOwner::Entity(9));
        assert!(tickets.insert(ticket("m", a, (0, 0))));
        assert!(!tickets.insert(ticket("m", a, (0, 0))));
        assert!(tickets.insert(ticket("m", b, (0, 0))));
        // The same owner under another mod is another ticket.
        assert!(tickets.insert(ticket("n", a, (0, 0))));
        assert!(tickets.remove(&ticket("m", a, (0, 0))));
        assert!(!tickets.remove(&ticket("m", a, (0, 0))));
        assert!(tickets.holds((0, 0)));
        assert_eq!(tickets.of_mod("m").count(), 1);
        assert!(tickets.remove(&ticket("m", b, (0, 0))));
        assert!(tickets.remove(&ticket("n", a, (0, 0))));
        assert!(!tickets.holds((0, 0)));
        assert_eq!(tickets.chunks().count(), 0);
    }

    #[test]
    fn tickets_of_mods_not_loaded_are_discarded() {
        let saved = vec![
            ticket("here", TicketOwner::Entity(1), (4, 4)),
            ticket("gone", TicketOwner::Block([0, 0, 0]), (8, 8)),
        ];
        let tickets = ModTickets::from_saved(saved, |id| id == "here");
        assert!(tickets.holds((4, 4)));
        assert!(!tickets.holds((8, 8)));
        assert_eq!(tickets.to_save().count(), 1);
    }

    fn in_mod(id: &'static str) -> crate::crash::NativeModGuard {
        crate::crash::enter_native_mod(crate::crash::NativeModCall {
            id,
            hook: "test",
            part: None,
        })
    }

    #[test]
    fn validation_shows_and_removes_only_the_callers_tickets() {
        let (block, entity) = (TicketOwner::Block([1, 2, 3]), TicketOwner::Entity(9));
        let mut v = TicketValidation::new(vec![
            ticket("m", block, (0, 0)),
            ticket("m", block, (0, 1)),
            ticket("m", entity, (5, 5)),
            ticket("n", block, (0, 0)),
        ]);
        assert!(v.block_tickets().is_empty(), "no caller, no tickets");
        let guard = in_mod("m");
        assert_eq!(v.block_tickets(), vec![([1, 2, 3], vec![[0, 0], [0, 1]])]);
        assert_eq!(v.entity_tickets(), vec![(9, vec![[5, 5]])]);
        assert!(v.remove_block_ticket([1, 2, 3], [0, 1]));
        assert!(!v.remove_block_ticket([1, 2, 3], [0, 1]));
        assert_eq!(v.remove_entity_tickets(9), 1);
        assert_eq!(v.remove_block_tickets([1, 2, 3]), 1);
        drop(guard);
        let _guard = in_mod("n");
        assert_eq!(v.remove_block_tickets([1, 2, 3]), 1, "its own, not m's");
        assert_eq!(v.into_removed().len(), 4);
    }
}
