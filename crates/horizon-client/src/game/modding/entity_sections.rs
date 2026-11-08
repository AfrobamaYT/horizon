//! `entity_entering_section` (`EntityEvent.EnteringSection`): vanilla tells it from
//! `Entity.setPosRaw` whenever an entity's section changes, on both sides. Horizon has no one
//! place where positions are set (every kind moves its own, much of it in client-lib), so the
//! hook is told from one tracker instead: each side looks at every entity of the level it shows
//! once a tick, through [`Game::native_entity_sections`], and tells those whose section changed
//! since the last look. The server looks at the end of each level's tick, at every kind in the
//! level's lists and at its players; a client after its tick, at its own player and at every
//! entity its server told it of. Each level keeps its own tracker, so an entity changing level
//! starts untold in the new one, as vanilla adds it to its new section without the event.
//!
//! With no listener a look is one subscription check. With one, it is a hash lookup per entity
//! and nothing is allocated once the table has grown to the level's entities.

use super::Game;
use super::entity::{Entity as _, entity_lists_of, for_each_entity};
use mod_api::{EntityRef, NativeWorldEpoch};
use std::collections::HashMap;

#[cfg(test)]
#[path = "entity_sections/tests.rs"]
mod tests;

/// `SectionPos.blockToSectionCoord(Mth.floor(v))`: `Mth.floor` truncates (saturating, NaN to 0,
/// as Java's cast does) and steps down for a negative fraction with Java's wrapping `int`.
fn section_coord(v: f64) -> i32 {
    let truncated = v as i32;
    let block = if v < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    };
    block >> 4
}

fn section_of([x, y, z]: [f64; 3]) -> [i32; 3] {
    [section_coord(x), section_coord(y), section_coord(z)]
}

/// An entity's section as of the look that last saw it.
struct Seen {
    section: [i32; 3],
    look: u32,
}

/// A section change one look found: the entity, its old and its new section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Crossing {
    pub(super) id: u32,
    pub(super) old: [i32; 3],
    pub(super) new: [i32; 3],
}

/// The section of each entity one level held at the last look.
#[derive(Default)]
pub(super) struct EntitySections {
    /// The world open (or client level) the table was filled in: another one starts it over.
    epoch: Option<NativeWorldEpoch>,
    seen: HashMap<u32, Seen>,
    look: u32,
    /// This look's crossings, in the order the entities were seen; kept for its capacity.
    crossed: Vec<Crossing>,
}

impl EntitySections {
    /// A look starts, in the world of `epoch`.
    fn begin(&mut self, epoch: Option<NativeWorldEpoch>) {
        if self.epoch != epoch {
            self.epoch = epoch;
            self.seen.clear();
        }
        self.look = self.look.wrapping_add(1);
        self.crossed.clear();
    }

    /// Entity `id` is at `position`: a crossing when the last look saw it in another section.
    /// One seen for the first time is only recorded.
    fn see(&mut self, id: u32, position: [f64; 3]) {
        let section = section_of(position);
        let look = self.look;
        match self.seen.entry(id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let seen = entry.get_mut();
                seen.look = look;
                if seen.section != section {
                    self.crossed.push(Crossing {
                        id,
                        old: seen.section,
                        new: section,
                    });
                    seen.section = section;
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Seen { section, look });
            }
        }
    }

    /// The look ends: an entity it did not see has left the level, and is new if it returns.
    fn end(&mut self) {
        let look = self.look;
        self.seen.retain(|_, seen| seen.look == look);
    }

    /// `id` starts over where it is next seen, untold (a player's respawn: a new entity under
    /// the same id).
    pub(super) fn forget(&mut self, id: u32) {
        self.seen.remove(&id);
    }
}

impl Game<'_> {
    /// Look at every entity of the level this side shows and tell `entity_entering_section`
    /// each section change since the last look. The server calls it at the end of each
    /// level's tick, a client after its own.
    pub(super) fn native_entity_sections(&mut self) {
        if !mod_api::__private::listened(mod_api::HookId::EntityEnteringSection) {
            return;
        }
        let epoch = self.native_lifecycle.epoch();
        let mut sections = std::mem::take(&mut self.entity_sections);
        sections.begin(epoch);
        if self.is_logical_server() {
            self.see_server_entities(&mut sections);
        } else {
            self.see_client_entities(&mut sections);
        }
        sections.end();
        let mut crossed = std::mem::take(&mut sections.crossed);
        self.entity_sections = sections;
        if let Some(epoch) = epoch {
            for c in &crossed {
                let chunk_changed = c.old[0] != c.new[0] || c.old[2] != c.new[2];
                self.native_entity_entering_section(
                    EntityRef::new(epoch, c.id),
                    c.old,
                    c.new,
                    chunk_changed,
                );
            }
        }
        // A listener's own moves are seen at the next look; the buffer goes back for reuse.
        crossed.clear();
        self.entity_sections.crossed = crossed;
    }

    /// The level in place's entities: every kind in its lists, the kinds kept apart, the cast
    /// fishing hooks and the players in it. Paintings and item frames are left out: they never
    /// move once hung (`/tp` and mods refuse them), and a dragon's parts are not entities of the
    /// level's sections.
    fn see_server_entities(&self, sections: &mut EntitySections) {
        let mut see = |id: u32, at: [f64; 3]| {
            if id != 0 {
                sections.see(id, at);
            }
        };
        for_each_entity!(entity_lists_of!(self), entity => {
            if !entity.is_removed() {
                let at = entity.position();
                see(entity.id(), [at.x, at.y, at.z]);
            }
        });
        for o in self.others.iter() {
            see(o.id, [o.position.x, o.position.y, o.position.z]);
        }
        for c in self.cushions.iter() {
            see(c.id, [c.position.x, c.position.y, c.position.z]);
        }
        for f in self.fangs.list.iter() {
            see(f.id, f.pos);
        }
        for c in self.crystals.iter() {
            see(c.id, [c.position.x, c.position.y, c.position.z]);
        }
        if let Some(d) = self.dragon.as_ref() {
            see(d.id, [d.position.x, d.position.y, d.position.z]);
        }
        for s in self.ominous_spawners.iter() {
            see(s.id, s.position);
        }
        for c in self.breath_clouds.iter() {
            see(c.id, c.at);
        }
        for (hook, _) in self.fishing_hooks_here() {
            see(
                hook.id(),
                [hook.position.x, hook.position.y, hook.position.z],
            );
        }
        if let Some(host) = self.lan.as_ref() {
            for g in host.hub.guests.iter().filter(|g| host.hub.here(g)) {
                see(g.entity_id as u32, g.position);
            }
        }
    }

    /// A client's own player, and every entity its server told it of where the server last
    /// placed it (its move, teleport and sync packets).
    fn see_client_entities(&self, sections: &mut EntitySections) {
        let Some(remote) = self.remote.as_ref() else {
            return;
        };
        let own = remote.entity_id;
        for (id, at) in remote.tracked_positions() {
            if id != own && id != 0 {
                sections.see(id as u32, at);
            }
        }
        if let Some(player) = self.player.as_ref()
            && own != 0
        {
            let at = player.position;
            sections.see(own as u32, [at.x, at.y, at.z]);
        }
    }

    /// A player respawned as a new entity under the same id: it starts untold where it comes
    /// back, in whichever level that is.
    pub(super) fn forget_entity_section(&mut self, id: u32) {
        self.entity_sections.forget(id);
        self.levels.forget_entity_section(id);
    }
}
