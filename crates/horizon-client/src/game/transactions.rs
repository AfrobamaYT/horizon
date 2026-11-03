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
//! cauldron block states. A block's tentative state is stored only (`World::set_stored`): the
//! one real write, with its remesh, relight and logs, is at the root commit. A jukebox is
//! told at the root commit from its record before and after, so a disc swapped in one
//! transaction changes the song. A player's inventory (slots, armour, off hand, cursor) is kept
//! whole on its first write, from wherever the player's state lives then: the local player's or
//! a guest's, swapped in by `native_as_player`. An item a write drops from a player waits for
//! the root commit too.
use super::{BlockEntity, BlockStateId, Game, machines::HopperState};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::TransactionSeal;
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
    /// Each player's inventory before this transaction first wrote it, by UUID.
    players: HashMap<u128, KeptPlayer>,
    /// Each entity's inventory (`entity_slots`) before this transaction first wrote it.
    entities: HashMap<u32, Vec<Option<ItemStack>>>,
    /// What the root commit does, in order.
    after: Vec<After>,
}

struct Kept {
    entity: Option<BlockEntity>,
    /// None where it had no map: an abort removes one a write made.
    attachments: Option<Attachments>,
    hopper: Option<HopperState>,
}

#[derive(Clone)]
struct KeptPlayer {
    inventory: Vec<Option<ItemStack>>,
    armor: [Option<ItemStack>; 4],
    offhand: Option<ItemStack>,
    carried: Option<ItemStack>,
}

/// What a write tells the world, at the root commit.
pub(super) enum After {
    /// `BlockEntity.setChanged`.
    Changed(Pos),
    /// The block's new state, set with its updates (`Level.setBlock`, `UPDATE_ALL`).
    Block(Pos),
    /// `Block.popResource`.
    Drop([i32; 3], ItemStack),
    /// `Player.drop(stack, false)` by the player with this UUID: what no longer fits.
    PlayerDrop(u128, ItemStack),
}

impl Game<'_> {
    /// Opens a transaction, nested in the open one if any (`Transaction.open`). Only
    /// `host.transaction` reaches it, through its seal.
    pub(in crate::game) fn native_transaction_open(&mut self, seal: TransactionSeal) {
        let _ = seal;
        self.transaction_open();
    }
    /// `native_transaction_close`, which only `host.transaction` reaches, through its seal.
    pub(in crate::game) fn native_transaction_close(
        &mut self,
        seal: TransactionSeal,
        commit: bool,
    ) {
        let _ = seal;
        self.transaction_close(commit);
    }
    fn transaction_open(&mut self) {
        self.transactions.push(Level::default());
    }
    /// Closes the innermost transaction: `commit` hands its changes to the one around it, or
    /// at the root makes them final and tells the world; otherwise every storage it wrote goes
    /// back to how it was. Nothing happens with none open.
    fn transaction_close(&mut self, commit: bool) {
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
                for (uuid, kept) in level.players {
                    outer.players.entry(uuid).or_insert(kept);
                }
                for (id, kept) in level.entities {
                    outer.entities.entry(id).or_insert(kept);
                }
                outer.after.extend(level.after);
            }
            (true, None) => self.root_commit(level),
            (false, _) => self.roll_back(level),
        }
    }
    /// Closes every transaction left open, as aborted. Run before each tick, as a backstop:
    /// `host.transaction` is the only way in, and its drop guard closes each one it opens, so
    /// none is left open by a mod.
    pub(super) fn abort_open_transactions(&mut self) {
        while !self.transactions.is_empty() {
            self.transaction_close(false);
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
    /// Keeps the inventory of the player whose state is in `Game` now, `uuid`, as it is, if a
    /// transaction is open and hasn't yet. Called inside `native_as_player`.
    pub(super) fn journal_player(&mut self, uuid: u128) {
        let Some(level) = self.transactions.last_mut() else {
            return;
        };
        level.players.entry(uuid).or_insert_with(|| KeptPlayer {
            inventory: self.inventory.clone(),
            armor: self.armor.clone(),
            offhand: self.offhand.clone(),
            carried: self.carried.clone(),
        });
    }
    /// Keeps entity `id`'s inventory as it is, if a transaction is open and hasn't yet.
    pub(super) fn journal_entity(&mut self, id: u32) {
        if self.transactions.is_empty() {
            return;
        }
        let Some((slots, _)) = self.entity_slots(id) else {
            return;
        };
        let slots = slots.to_vec();
        if let Some(level) = self.transactions.last_mut() {
            level.entities.entry(id).or_insert(slots);
        }
    }
    /// `drop_plain` by the player whose state is in `Game` now, `uuid`: at once, or at the root
    /// commit inside a transaction.
    pub(super) fn drop_after_commit(&mut self, uuid: u128, stack: ItemStack) {
        match self.transactions.last_mut() {
            Some(level) => level.after.push(After::PlayerDrop(uuid, stack)),
            None => self.drop_plain(stack),
        }
    }
    /// Sets the block at `pos` to `state`. In a transaction, stored only, keeping the old
    /// state, and set with its updates at the root commit; otherwise at once
    /// (`Level.setBlock`).
    pub(super) fn journal_set_block(&mut self, pos: [i32; 3], state: BlockStateId) -> bool {
        let Some(level) = self.transactions.last_mut() else {
            return self.level_set(pos, state).is_ok();
        };
        let key = (pos[0], pos[1], pos[2]);
        let old = self.world.block(pos[0], pos[1], pos[2]);
        level.blocks.entry(key).or_insert(old);
        level.after.push(After::Block(key));
        self.world.set_stored(pos[0], pos[1], pos[2], state)
    }
    /// Whether a transaction is open: a jukebox's song then waits for the root commit.
    pub(super) fn in_transaction(&self) -> bool {
        !self.transactions.is_empty()
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
            // Only reached through a root commit, which sets the block itself.
            After::Block(_) => {}
            After::Drop(pos, stack) => {
                self.spawn_item(stack, pos.map(|p| f64::from(p) + 0.5), [0.0; 3], 10);
            }
            After::PlayerDrop(uuid, stack) => {
                // A player who left meanwhile drops nothing; their inventory had changed for good.
                self.native_as_player(uuid, |g| g.drop_plain(stack));
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
                        // The stored state back, so the one real write sees the change.
                        self.world.set_stored(pos.0, pos.1, pos.2, was);
                        let _ = self.level_set([pos.0, pos.1, pos.2], now);
                    }
                }
                after => self.run_after(after),
            }
        }
        for (pos, kept) in level.block_entities {
            let record = |entity: Option<&BlockEntity>| match entity {
                Some(BlockEntity::Jukebox(j)) => j.record().cloned(),
                _ => None,
            };
            let was = record(kept.entity.as_ref());
            let now = record(self.world.block_entities.get(&pos));
            if was == now {
                continue;
            }
            // The song of the record it had stops; the one it has now starts.
            let pos = [pos.0, pos.1, pos.2];
            if was.is_some() {
                self.jukebox_stopped(pos);
            }
            if now.is_some() {
                self.jukebox_inserted(pos);
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
            self.world
                .block_entities
                .set_attachments(pos, kept.attachments);
            match kept.hopper {
                Some(hopper) => self.hoppers.insert(pos, hopper),
                None => self.hoppers.remove(&pos),
            };
        }
        for (pos, state) in level.blocks {
            self.world.set_stored(pos.0, pos.1, pos.2, state);
        }
        for (id, kept) in level.entities {
            if let Some((slots, _)) = self.entity_slots(id) {
                for (slot, kept) in slots.iter_mut().zip(kept) {
                    *slot = kept;
                }
            }
        }
        for (uuid, kept) in level.players {
            self.native_as_player(uuid, |g| {
                g.inventory = kept.inventory;
                g.armor = kept.armor;
                g.offhand = kept.offhand;
                g.carried = kept.carried;
            });
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
        let jukebox = registry.default_state("minecraft:jukebox").unwrap();
        let flags = NativeBlockUpdateFlags::NEIGHBORS;
        assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
        assert!(game.native_level_set_block(level, [2, 64, 0], full, flags));
        assert!(game.native_level_set_block(level, [4, 64, 0], jukebox, flags));
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

        // A transaction a mod opened through the helpers and never closed is aborted before
        // the next tick.
        game.transaction_open();
        let host: &mut dyn Host = &mut game;
        host.container_insert([0, 64, 0], None, 0, sticks(1), false);
        game.abort_open_transactions();
        assert_eq!(chest_items(&game), [(0, 3)]);

        // A panic caught outside the root: what it changed is gone before the catch returns,
        // and the transaction is closed.
        let host: &mut dyn Host = &mut game;
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            host.transaction(|host| -> Result<(), ()> {
                host.container_insert([0, 64, 0], None, 0, sticks(1), false);
                panic!("a broken handler")
            })
        }));
        assert!(caught.is_err());
        assert_eq!(chest_items(&game), [(0, 3)]);
        assert!(game.transactions.is_empty());

        // An attachment map a write made inside an aborted transaction is gone with it.
        let chest_holder = horizon_client::mod_api::Holder::BlockEntity([0, 64, 0]);
        assert!(game.world.block_entities.attachments((0, 64, 0)).is_none());
        game.transaction_open();
        assert!(game.attachment_map_quiet(chest_holder).is_ok());
        assert!(game.world.block_entities.attachments((0, 64, 0)).is_some());
        game.transaction_close(false);
        assert!(game.world.block_entities.attachments((0, 64, 0)).is_none());

        // An aborted cauldron change leaves no write behind in the world's queues.
        assert!(game.native_level_set_block(level, [2, 64, 0], full, flags));
        game.world.block_log = Some(Vec::new());
        game.world.path_writes = Some(Vec::new());
        let queues = |game: &Game<'_>| {
            let world = &game.world;
            (
                world.block_log.as_ref().map(Vec::len),
                world.path_writes.as_ref().map(Vec::len),
                world.light_edits.len(),
                world.dirty_light.len(),
                world.watched_writes.len(),
            )
        };
        let before = queues(&game);
        let host: &mut dyn Host = &mut game;
        let _ = host.transaction(|host| {
            host.fluid_tank_extract([2, 64, 0], None, 0, 1000, false);
            Err::<(), ()>(())
        });
        assert_eq!(game.world.block(2, 64, 0), full);
        assert_eq!(queues(&game), before);

        // A disc swapped for another in one transaction changes the song: the old one's
        // stops and the new one's starts.
        let disc = |item: &str| ItemStack::new(item, 1);
        let host: &mut dyn Host = &mut game;
        host.container_insert([4, 64, 0], None, 0, disc("minecraft:music_disc_cat"), false);
        let Some(BlockEntity::Jukebox(j)) = game.world.block_entities.get_mut(&(4, 64, 0)) else {
            panic!("jukebox block entity missing")
        };
        assert!(j.playing, "a disc put in through a transfer plays");
        j.ticks = 500;
        let host: &mut dyn Host = &mut game;
        let swapped = host.transaction(|host| {
            host.container_extract([4, 64, 0], None, 0, 1, false);
            Ok::<_, ()>(host.container_insert(
                [4, 64, 0],
                None,
                0,
                disc("minecraft:music_disc_13"),
                false,
            ))
        });
        assert_eq!(swapped, Ok(None));
        let Some(BlockEntity::Jukebox(j)) = game.world.block_entities.get(&(4, 64, 0)) else {
            panic!("jukebox block entity missing")
        };
        assert_eq!(
            j.record().map(|r| &*r.item),
            Some("minecraft:music_disc_13")
        );
        assert!(j.playing);
        assert_eq!(j.ticks, 0, "the new disc's song starts from its beginning");

        // A player's place: a bucket of three in the hand splits one off, filled; the hand
        // keeps two and the filled one goes to the inventory, the cursor changes too, and what
        // doesn't fit would be thrown. Aborted, the hand, the inventory and the cursor are as
        // they were, and nothing was thrown.
        game.player = Some(
            super::super::PlayerMotion::new(
                game.motion,
                horizon_core::geometry::Vec3::new(0.5, 65.0, 3.5),
            )
            .unwrap(),
        );
        let bucket = |item: &str, count| ItemStack::new(item, count);
        game.inventory
            .iter_mut()
            .for_each(|slot| *slot = Some(bucket("minecraft:stone", 64)));
        game.selected = 0;
        game.inventory[0] = Some(bucket("minecraft:bucket", 3));
        game.carried = None;
        let before = (
            game.inventory.clone(),
            game.offhand.clone(),
            game.carried.clone(),
        );
        let dropped = game.dropped.len();
        let me = game.native_acting_id();
        let hand = horizon_client::mod_api::ItemPlace::PlayerHand {
            player: me,
            hand: horizon_client::first_person::Hand::Main,
        };
        let cursor = horizon_client::mod_api::ItemPlace::PlayerCursor { player: me };
        let host: &mut dyn Host = &mut game;
        let _ = host.transaction(|host| {
            let water = bucket("minecraft:water_bucket", 1);
            assert!(host.set_place_stack(&hand, water, true, false).is_none());
            assert!(
                host.set_place_stack(&cursor, bucket("minecraft:stick", 4), false, false)
                    .is_none()
            );
            Err::<(), ()>(())
        });
        assert_eq!(
            (
                game.inventory.clone(),
                game.offhand.clone(),
                game.carried.clone()
            ),
            before
        );
        assert_eq!(
            game.dropped.len(),
            dropped,
            "a thrown bucket waits for the root commit"
        );

        // Committed, the split stands and the full inventory throws the filled bucket.
        let host: &mut dyn Host = &mut game;
        let _ = host.transaction(|host| {
            let water = bucket("minecraft:water_bucket", 1);
            host.set_place_stack(&hand, water, true, false);
            Ok::<_, ()>(())
        });
        assert_eq!(game.inventory[0], Some(bucket("minecraft:bucket", 2)));
        assert_eq!(game.dropped.len(), dropped + 1);
        let thrown = &game.dropped[game.dropped.len() - 1].stack;
        assert_eq!(&*thrown.item, "minecraft:water_bucket");

        // A chest minecart's, a chest boat's and a chested donkey's slots: an abort puts back
        // what a write and a split changed; a commit keeps it.
        let mut chested = horizon_world::nbt::Compound::new();
        chested.insert("ChestedHorse".into(), horizon_world::nbt::Tag::Byte(1));
        let vehicles = [
            (
                "minecraft:chest_minecart",
                horizon_world::nbt::Compound::new(),
            ),
            (
                "minecraft:oak_chest_boat",
                horizon_world::nbt::Compound::new(),
            ),
            ("minecraft:donkey", chested),
        ];
        for (kind, nbt) in vehicles {
            let id = game
                .native_summon("minecraft:overworld", kind, [8.5, 70.0, 8.5], nbt)
                .unwrap_or_else(|| panic!("{kind} not summoned"));
            let place = |slot| horizon_client::mod_api::ItemPlace::EntitySlot { entity: id, slot };
            let host: &mut dyn Host = &mut game;
            assert!(
                host.set_place_stack(&place(0), sticks(5), false, false)
                    .is_none(),
                "{kind}"
            );
            let _ = host.transaction(|host| {
                host.set_place_stack(&place(0), sticks(9), false, false);
                // From five, one splits off changed into the next free slot.
                host.set_place_stack(&place(1), sticks(2), false, false);
                Err::<(), ()>(())
            });
            let host: &mut dyn Host = &mut game;
            assert_eq!(host.place_stack(&place(0)), Some(sticks(5)), "{kind}");
            assert_eq!(host.place_stack(&place(1)), None, "{kind}");
            let _ = host.transaction(|host| {
                let one = ItemStack::new("minecraft:blaze_rod", 1);
                host.set_place_stack(&place(0), one, true, false);
                Ok::<_, ()>(())
            });
            let host: &mut dyn Host = &mut game;
            assert_eq!(host.place_stack(&place(0)), Some(sticks(4)), "{kind}");
            let rod = host.place_stack(&place(1)).map(|s| s.item);
            assert_eq!(rod.as_deref(), Some("minecraft:blaze_rod"), "{kind}");
        }
    }
}
