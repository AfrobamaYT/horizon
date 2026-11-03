//! `host.transaction` (#41 §7): NeoForge 26.3's `Transaction` and `SnapshotJournal`, kept by
//! the game at the storage level, so a mod's handlers hold no state and write no journal.
//!
//! A write through a capability helper inside a transaction applies in place, so later steps
//! see it; the first write to a storage at each depth keeps that storage as it was. What a
//! write would tell the world (`setChanged` and comparators, a jukebox starting or stopping, a
//! cauldron's block update, an item dropped) waits for the root commit and then runs once,
//! as `SnapshotJournal.onRootCommit` does. A nested commit hands its snapshots and waiting
//! work to the transaction around it; an abort puts every storage back quietly and drops the
//! waiting work, which nothing outside had seen.
//!
//! Covered here: block entities (slots, saved compound, attachments), a hopper's cooldown and
//! cauldron block states. Player and entity inventories come with item places.
use super::{BlockEntity, BlockStateId, Game, machines::HopperState};
use horizon_client::items::ItemStack;
use horizon_world::attachments::Attachments;
use std::collections::HashMap;

type Pos = (i32, i32, i32);

/// One open transaction.
#[derive(Default)]
pub(super) struct Level {
    /// Each block entity as it was before this transaction first wrote it.
    block_entities: HashMap<Pos, Kept>,
    /// Each block's state before this transaction first wrote it.
    blocks: HashMap<Pos, BlockStateId>,
    /// What the root commit does, in order.
    after: Vec<After>,
}

struct Kept {
    entity: Option<BlockEntity>,
    attachments: Option<Attachments>,
    hopper: Option<HopperState>,
}

/// What a write tells the world, at the root commit.
pub(super) enum After {
    /// `BlockEntity.setChanged`.
    Changed(Pos),
    /// `JukeboxBlockEntity.setTheItem` through a transfer: the disc starts.
    JukeboxInserted([i32; 3]),
    /// `JukeboxBlockEntity.removeItem`: the song stops.
    JukeboxTaken([i32; 3]),
    /// The block's new state, set with its updates (`Level.setBlock`, `UPDATE_ALL`).
    Block(Pos),
    /// `Block.popResource`.
    Drop([i32; 3], ItemStack),
}

impl Game<'_> {
    /// Opens a transaction, nested in the open one if any (`Transaction.open`).
    pub(in crate::game) fn native_transaction_open(&mut self) {
        self.transactions.push(Level::default());
    }
    /// Closes the innermost transaction: `commit` hands its changes to the one around it, or
    /// at the root makes them final and tells the world; otherwise every storage it wrote goes
    /// back to how it was. Nothing happens with none open.
    pub(in crate::game) fn native_transaction_close(&mut self, commit: bool) {
        let Some(level) = self.transactions.pop() else {
            return;
        };
        match (commit, self.transactions.last_mut()) {
            (true, Some(outer)) => {
                for (pos, kept) in level.block_entities {
                    outer.block_entities.entry(pos).or_insert(kept);
                }
                for (pos, state) in level.blocks {
                    outer.blocks.entry(pos).or_insert(state);
                }
                outer.after.extend(level.after);
            }
            (true, None) => self.root_commit(level),
            (false, _) => self.roll_back(level),
        }
    }
    /// Closes every transaction left open, as aborted: a panic inside one unwound past its
    /// close. Run before each tick.
    pub(super) fn abort_open_transactions(&mut self) {
        while !self.transactions.is_empty() {
            self.native_transaction_close(false);
        }
    }
    /// Keeps the block entity at `pos` as it is, with its attachments and a hopper's
    /// cooldown, if a transaction is open and hasn't yet.
    pub(super) fn journal_block_entity(&mut self, pos: Pos) {
        let Some(level) = self.transactions.last_mut() else {
            return;
        };
        level.block_entities.entry(pos).or_insert_with(|| Kept {
            entity: self.world.block_entities.get(&pos).cloned(),
            attachments: self.world.block_entities.attachments(pos).cloned(),
            hopper: self.hoppers.get(&pos).copied(),
        });
    }
    /// Sets the block at `pos` to `state`. In a transaction, quietly, keeping the old state,
    /// with its updates at the root commit; otherwise at once (`Level.setBlock`).
    pub(super) fn journal_set_block(&mut self, pos: [i32; 3], state: BlockStateId) -> bool {
        let Some(level) = self.transactions.last_mut() else {
            return self.level_set(pos, state).is_ok();
        };
        let key = (pos[0], pos[1], pos[2]);
        let old = self.world.block(pos[0], pos[1], pos[2]);
        level.blocks.entry(key).or_insert(old);
        level.after.push(After::Block(key));
        self.world.set(pos[0], pos[1], pos[2], state).is_ok()
    }
    /// Runs `after` at once, or at the root commit inside a transaction.
    pub(super) fn after_commit(&mut self, after: After) {
        match self.transactions.last_mut() {
            Some(level) => level.after.push(after),
            None => self.run_after(after),
        }
    }
    fn run_after(&mut self, after: After) {
        match after {
            After::Changed(pos) => self.container_changed(pos),
            After::JukeboxInserted(pos) => self.jukebox_inserted(pos),
            After::JukeboxTaken(pos) => self.jukebox_taken(pos),
            // Only reached through a root commit, which sets the block itself.
            After::Block(_) => {}
            After::Drop(pos, stack) => {
                self.spawn_item(stack, pos.map(|p| f64::from(p) + 0.5), [0.0; 3], 10);
            }
        }
    }
    fn root_commit(&mut self, level: Level) {
        let mut told = std::collections::HashSet::new();
        for after in level.after {
            match after {
                // Each storage tells the world once, after every write.
                After::Changed(pos) if !told.insert(Ok(pos)) => {}
                After::Block(pos) => {
                    if !told.insert(Err(pos)) {
                        continue;
                    }
                    let now = self.world.block(pos.0, pos.1, pos.2);
                    let was = level.blocks[&pos];
                    if now != was {
                        // Back to the old state quietly, so the set sees the change.
                        let _ = self.world.set(pos.0, pos.1, pos.2, was);
                        let _ = self.level_set([pos.0, pos.1, pos.2], now);
                    }
                }
                after => self.run_after(after),
            }
        }
    }
    fn roll_back(&mut self, level: Level) {
        for (pos, kept) in level.block_entities {
            if let (Some(entity), Some(now)) =
                (kept.entity, self.world.block_entities.get_mut(&pos))
            {
                *now = entity;
            }
            if let (Some(attachments), Some(now)) = (
                kept.attachments,
                self.world.block_entities.attachments_mut(pos),
            ) {
                *now = attachments;
            }
            match kept.hopper {
                Some(hopper) => self.hoppers.insert(pos, hopper),
                None => self.hoppers.remove(&pos),
            };
        }
        for (pos, state) in level.blocks {
            let _ = self.world.set(pos.0, pos.1, pos.2, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Audio, BlockEntity, Game, Options, Progress, Settings, load_resources};
    use horizon_client::{
        chunk::Chunk,
        items::ItemStack,
        mod_api::{Host, NativeAuthority, NativeBlockUpdateFlags},
    };
    use std::{sync::Arc, time::Instant};

    fn chest_items(game: &Game<'_>) -> Vec<(usize, u32)> {
        let Some(BlockEntity::Chest(items)) = game.world.block_entities.get(&(0, 64, 0)) else {
            panic!("chest block entity missing")
        };
        let held = items.iter().enumerate();
        held.filter_map(|(i, s)| Some((i, s.as_ref()?.count)))
            .collect()
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn an_aborted_transaction_puts_back_what_it_and_its_committed_children_changed() {
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
        options.dedicated = Some(Vec::new());
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
        let registry = &game.pack.assets.registry;
        let chest = registry.default_state("minecraft:chest").unwrap();
        let full = horizon_world::chunk::BlockState {
            name: "minecraft:water_cauldron".to_owned(),
            properties: [("level".to_owned(), "3".to_owned())].into_iter().collect(),
        };
        let full = registry.resolve(&full).unwrap();
        let empty = registry.default_state("minecraft:cauldron").unwrap();
        let flags = NativeBlockUpdateFlags::NEIGHBORS;
        assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
        assert!(game.native_level_set_block(level, [2, 64, 0], full, flags));
        let sticks = |count| ItemStack::new("minecraft:stick", count);
        let host: &mut dyn Host = &mut game;

        // Five in, then a committed child takes two and an aborted one adds three: the
        // outer abort puts all of it back, the cauldron's water too.
        let refused: Result<(), ()> = host.transaction(|host| {
            assert!(
                host.container_insert([0, 64, 0], None, 0, sticks(5), false)
                    .is_none()
            );
            let taken = host.transaction(|host| {
                Ok::<_, ()>(host.container_extract([0, 64, 0], None, 0, 2, false))
            });
            assert_eq!(taken, Ok(Some(sticks(2))));
            let _ = host.transaction(|host| {
                host.container_insert([0, 64, 0], None, 1, sticks(3), false);
                Err::<(), ()>(())
            });
            let water = host.fluid_tank_extract([2, 64, 0], None, 0, 1000, false);
            assert_eq!(water.map(|w| w.amount), Some(1000));
            Err(())
        });
        assert_eq!(refused, Err(()));
        assert_eq!(chest_items(&game), []);
        assert_eq!(game.world.block(2, 64, 0), full);
        assert!(game.transactions.is_empty());

        // The same, committed: what the children left stands, the cauldron is set for good.
        let host: &mut dyn Host = &mut game;
        let done: Result<(), ()> = host.transaction(|host| {
            host.container_insert([0, 64, 0], None, 0, sticks(5), false);
            let _ = host.transaction(|host| {
                host.container_extract([0, 64, 0], None, 0, 2, false);
                Ok::<_, ()>(())
            });
            let _ = host.transaction(|host| {
                host.container_insert([0, 64, 0], None, 1, sticks(3), false);
                Err::<(), ()>(())
            });
            host.fluid_tank_extract([2, 64, 0], None, 0, 1000, false);
            Ok(())
        });
        assert_eq!(done, Ok(()));
        assert_eq!(chest_items(&game), [(0, 3)]);
        assert_eq!(game.world.block(2, 64, 0), empty);

        // A transaction a panic left open is aborted before the next tick.
        game.native_transaction_open();
        let host: &mut dyn Host = &mut game;
        host.container_insert([0, 64, 0], None, 0, sticks(1), false);
        game.abort_open_transactions();
        assert_eq!(chest_items(&game), [(0, 3)]);
    }
}
