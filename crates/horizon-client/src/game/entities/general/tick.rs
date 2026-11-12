//! PURPOSE: the general entity rows' authoritative tick (#792 stage 4): one
//! pass over the rows in join order, each root's rider subtree inside its
//! tick.
//! WHY: vanilla `ServerLevel.tick` walks `entityTickList.forEach`, and
//! `tickNonPassenger` runs the entity's common tick, its tick, then each
//! rider recursively (`tickPassenger`: a rider is never a root). The tick
//! list is insertion-ordered, and its `ensureActiveIsNotIterated` keeps a
//! join made during the pass out of the pass, so Horizon's join order
//! (`join_seq.rs`) is that order and a row that joins mid-pass first ticks
//! the next pass.
//! REFS: vanilla `ServerLevel.tick`, `tickNonPassenger`, `tickPassenger`,
//! `EntityTickList`; `join_seq.rs`; `ordered_tick.rs`; `tools/rea_vanilla.py`
//! (`add` appends to the active map, `remove` takes out, and the passive
//! swap holds a mid-pass join for the next tick).
//! LOGS: none. READS-BY: `Game::tick_world`. ENDS-WHEN: every kind ticks
//! from the join-order walk.

use super::super::Game;
use super::{GeneralTypeCatalog, general_type_catalog};
use horizon_client::join_seq::JoinSeq;
use mod_api::EntityRef;

impl Game<'_> {
    /// The general rows' entity phase: roots in join order, each root's
    /// common bookkeeping and subscribed server behavior, then its rider
    /// subtree. A row that joined this tick ticks from the next (the tick
    /// list's passive swap), a rider never ticks as a root, rows outside
    /// entity-ticking chunks stay (`ServerLevel.tick`'s range gate), and a
    /// parked level never reaches here at all. A type slot resolved without
    /// subscribers costs its common tick and nothing else: no per-row type
    /// lookup, call, or event allocation.
    pub(in crate::game) fn tick_general(&mut self) -> horizon_client::Result<()> {
        if self.resident_entities.general.rows.is_empty() {
            return Ok(());
        }
        // `begin_ordered_ticks`' cutoff: a row numbered at or above `born`
        // joined during this tick and first ticks the next.
        let born = self.ordered.born;
        let catalog = general_type_catalog();
        // One bit per type slot, resolved once per pass. The rows keep plain
        // slots, so the walk itself never names a type until a part hears it.
        let subscribed = general_tick_subscribed(catalog);
        let epoch = self.native_lifecycle.epoch();
        let entity_ticking = self.entity_ticking_area();
        let mut round = self.native_tick_round();
        // The walk stays on the live list, as the ordered run's does: a
        // lifted pass would take every row's residency slot, and a part's
        // writes reach its row through that slot. What joins during the pass
        // lands at or above `born`, ticks from the next pass, and never
        // reaches this loop's rows.
        let total = self.resident_entities.general.rows.len();
        // Each vehicle's riders by mount link, in join order: a rider's place
        // in the storage is not its place in its vehicle's subtree. A tree
        // without riders collects nothing here.
        let mut riders: Vec<(u32, usize)> = Vec::new();
        for (index, row) in self.resident_entities.general.rows.iter().enumerate() {
            if let Some(vehicle) = row.mount {
                riders.push((vehicle, index));
            }
        }
        for index in 0..total {
            let (id, slot, join, mount, at) = {
                let row = &self.resident_entities.general.rows[index];
                (row.id, row.type_slot, row.join, row.mount, row.position)
            };
            // A rider ticks inside its vehicle's tick (`tickNonPassenger`
            // never takes one as a root); a row of this tick's join waits
            // for the next pass.
            if mount.is_some() || join >= born {
                continue;
            }
            // ServerLevel.tick: only entities in entity-ticking chunks tick.
            if !entity_ticking(at.x.floor() as i32, at.z.floor() as i32) {
                continue;
            }
            // `Entity.commonTick`'s setOldPosAndRot: the row's previous
            // position is its current one. The rest of the common tick
            // (tick counting, invulnerable time) has no row field yet.
            self.resident_entities.general.rows[index].previous =
                self.resident_entities.general.rows[index].position;
            let kind = catalog.key(slot);
            let ticks = !round.listening() || self.native_tick_pre_one(&mut round, id, kind);
            if ticks {
                if subscribed.get(slot).copied().unwrap_or(false)
                    && let Some(epoch) = epoch
                {
                    self.dispatch_general_tick(kind, epoch, id);
                }
                self.tick_general_riders(id, born, catalog, &subscribed, epoch, &riders);
            }
            if round.listening() {
                self.native_tick_post_one(&round, id, kind);
            }
        }
        Ok(())
    }

    /// `tickPassenger` for one vehicle's subtree: each rider's common tick
    /// and its own subscribed behavior, depth-first as vanilla recurses into
    /// the rider's riders. A rider rides its vehicle's tick: no Pre of its
    /// own, and it ticks wherever its vehicle does (the root already passed
    /// the chunk gate). A rider that joined this tick waits for the next
    /// pass with its root — a tree joins whole.
    fn tick_general_riders(
        &mut self,
        vehicle: u32,
        born: JoinSeq,
        catalog: &GeneralTypeCatalog,
        subscribed: &[bool],
        epoch: Option<mod_api::WorldEpoch>,
        riders: &[(u32, usize)],
    ) {
        for &(rider_vehicle, index) in riders {
            if rider_vehicle != vehicle {
                continue;
            }
            let (id, slot, join) = {
                let row = &self.resident_entities.general.rows[index];
                (row.id, row.type_slot, row.join)
            };
            if join >= born {
                continue;
            }
            self.resident_entities.general.rows[index].previous =
                self.resident_entities.general.rows[index].position;
            if subscribed.get(slot).copied().unwrap_or(false)
                && let Some(epoch) = epoch
            {
                self.dispatch_general_tick(catalog.key(slot), epoch, id);
            }
            self.tick_general_riders(id, born, catalog, subscribed, epoch, riders);
        }
    }

    /// One row's subscribed behavior, contained: the dispatch runs in a
    /// transaction, so a native panic unwinding out of it (caught here, as
    /// the mob tick's is) or a wasm trap inside it (a wasm dispatch fails
    /// without unwinding) rolls back every write the row's parts made —
    /// their counters and listener edits — and nothing leaves the row. The
    /// pass's lift stays balanced either way, and the next row ticks as if
    /// nothing happened; a panicking subscriber is reported, not disabled.
    fn dispatch_general_tick(&mut self, kind: &str, epoch: mod_api::WorldEpoch, id: u32) {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            mod_api::run_transaction(self, |game, _| {
                let before = game.wasm_dispatch_failures.get();
                game.native_general_entity_tick(kind, EntityRef::new(epoch, id));
                (game.wasm_dispatch_failures.get() == before)
                    .then_some(())
                    .ok_or(())
            })
        }));
        match outcome {
            Ok(Ok(())) => {}
            // A wasm trap: the runtime poisoned the caller's group and told
            // the log; the transaction above aborted with this refusal.
            Ok(Err(())) => {
                log::error!(
                    "general entity {kind} #{id}: its tick's writes were rolled back after a wasm dispatch failure"
                );
            }
            // A native panic: the unwind aborted the transaction as it
            // passed, and the thread-local mod attribution named the part.
            Err(payload) => {
                let panic = payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "box".to_owned());
                log::error!(
                    "general entity {kind} #{id}: its tick's writes were rolled back after a native mod panicked: {panic}"
                );
            }
        }
    }
}

/// Whether each type slot has a part subscribed to the tick hook: nothing
/// at all when no part anywhere subscribes (the dispatch's own bit check),
/// so a parts-free world resolves nothing.
fn general_tick_subscribed(catalog: &GeneralTypeCatalog) -> Vec<bool> {
    if !mod_api::__private::has_part_hook(mod_api::HookId::GeneralEntityTick) {
        return Vec::new();
    }
    (0..catalog.len())
        .map(|slot| {
            mod_api::__private::entity_part_hooked(
                mod_api::HookId::GeneralEntityTick,
                "general_entity_tick",
                catalog.key(slot),
            )
        })
        .collect()
}
