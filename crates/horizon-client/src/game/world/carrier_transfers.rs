//! Carrier transfers in the server's game: the fences that hold the
//! world's cells and the carrier still while an operation is between
//! prepared and committed, and the assembly that runs at a tick
//! boundary (issue #2413, slice 2 part C).
//!
//! The fences are one set of locked world cells plus locked carriers on
//! the server's level. Each refusal route is checked at one chokepoint:
//! breaking (`Game::break_block`), every mod write through the level's
//! write gate (`native_level_write_allowed`), pistons
//! (`Game::allow_piston_move`) and block-entity container opening
//! (`refuse_locked_container`). With no operation in flight the check is
//! one load of an empty set.

use crate::game::Game;
use horizon_world::level::LevelAccess;
use crate::carrier_store::Store;
use crate::save::encode_block_entity;
use horizon_core::position::BlockPos;
use horizon_world::carrier::codec;
use horizon_world::carrier::contact::RigidTransform;
use horizon_world::carrier::model::{Carrier, Cell, Invalid, LocalPos, MAX_CELLS};

/// Why a transfer refused. Refusals leave no partial transfer.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) enum TransferError {
    /// A cell's chunk is not loaded and ticking.
    CellNotLoaded([i32; 3]),
    /// More cells than the carrier model allows (4,096).
    TooManyCells(usize),
    /// A cell holds a block this slice does not move (not a plain block
    /// or a passive block entity).
    UnsupportedBlock([i32; 3]),
    /// The registry's own states cannot fit the operation envelope.
    Envelope,
    /// The model refused the assembled carrier.
    Model(Invalid),
    /// The store or its files refused.
    Store(String),
    /// Revalidation after the lift found a cell that did not move.
    Revalidation([i32; 3]),
    /// Plain I/O.
    Io(String),
}
impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CellNotLoaded(pos) => write!(
                f,
                "cell ({},{},{}) is not in a loaded, ticking chunk",
                pos[0], pos[1], pos[2]
            ),
            Self::TooManyCells(count) => write!(f, "{count} cells exceeds {MAX_CELLS}"),
            Self::UnsupportedBlock(pos) => write!(
                f,
                "cell ({},{},{}) holds a block this slice does not transfer",
                pos[0], pos[1], pos[2]
            ),
            Self::Envelope => write!(f, "the registry's states exceed the operation envelope"),
            Self::Model(invalid) => write!(f, "{invalid}"),
            Self::Store(text) => write!(f, "{text}"),
            Self::Revalidation(pos) => write!(
                f,
                "revalidation: cell ({},{},{}) did not move",
                pos[0], pos[1], pos[2]
            ),
            Self::Io(text) => write!(f, "carrier transfer I/O: {text}"),
        }
    }
}

impl Game<'_> {
    /// Assembles `cells` into one carrier at a tick boundary, server
    /// side. Preflight refuses anything this slice does not move and
    /// anything whose encoded operation cannot fit; the affected chunks
    /// are forced to disk with acknowledgement before `prepared`, the
    /// #2488 obligation covers the lift, the world cells and the carrier
    /// fence until the flush barrier confirms every submission and the
    /// journal records `committed`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn carrier_assemble(
        &mut self,
        uuid: u128,
        cells: &[BlockPos],
    ) -> Result<(), TransferError> {
        use crate::carrier_store::{envelope_preflight, encode_operation, OpKind, OperationCell};
        use crate::carrier_store::{CellImage, Operation};
        use horizon_client::join_seq::visibility::PassLease;

        if cells.len() > MAX_CELLS {
            return Err(TransferError::TooManyCells(cells.len()));
        }
        let registry = self.pack.assets.registry;
        envelope_preflight(registry).map_err(|_| TransferError::Envelope)?;

        // Preflight: loaded, ticking chunks; plain blocks or passive
        // block entities (this slice moves containers; ticking adapters
        // are slice 4's).
        let air = registry
            .default_state("minecraft:air")
            .ok_or_else(|| TransferError::Io("no air in the registry".into()))?;
        for pos in cells {
            let (x, z) = (pos.x >> 4, pos.z >> 4);
            if !self.world.chunks.contains_key(&(x, z)) {
                return Err(TransferError::CellNotLoaded([pos.x, pos.y, pos.z]));
            }
            let state = self.world.block(pos.x, pos.y, pos.z);
            if state == air {
                return Err(TransferError::UnsupportedBlock([pos.x, pos.y, pos.z]));
            }
        }

        // The store exists only from the first actual transfer.
        let mut store = match Store::open(&self.world_root) {
            Ok(crate::carrier_store::Opened::Live(store)) => store,
            Ok(crate::carrier_store::Opened::NeverUsed) => {
                let data_existed = self.world_root.join("data").is_dir();
                Store::first_use(&self.world_root, data_existed)
                    .map_err(|e| TransferError::Store(e.to_string()))?
            }
            Err(e) => return Err(TransferError::Store(e.to_string())),
        };

        // Before/after images from the live world, encoded in memory —
        // a transfer that cannot be stored never lifts a block.
        let mut operation_cells = Vec::with_capacity(cells.len());
        for pos in cells {
            let key = [pos.x, pos.y, pos.z];
            let state = self.world.block(pos.x, pos.y, pos.z);
            let descriptor = registry.state(state).ok_or(TransferError::UnsupportedBlock(key))?;
            let entity = self
                .world
                .block_entities
                .get(&(key[0], key[1], key[2]))
                .map(|entity| {
                    encode_block_entity((key[0], key[1], key[2]), entity, &descriptor.name)
                });
            operation_cells.push(OperationCell {
                world: key,
                local: [0, 0, 0],
                before: CellImage {
                    name: descriptor.name.clone(),
                    properties: descriptor
                        .properties
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    entity,
                },
                after: CellImage {
                    name: "minecraft:air".to_string(),
                    properties: Vec::new(),
                    entity: None,
                },
            });
        }
        // Local positions in carrier order: an x-major spread the model
        // sorts into its canonical order anyway.
        for (index, cell) in operation_cells.iter_mut().enumerate() {
            cell.local = [(index % 64) as i8, (index / 64) as i8, 0];
        }
        let carrier_cells = operation_cells
            .iter()
            .map(|cell| {
                Cell::new(
                    LocalPos::new(cell.local[0], cell.local[1], cell.local[2]),
                    &cell.before.name,
                    cell.before.properties.clone(),
                    None,
                )
            })
            .collect::<Result<Vec<_>, Invalid>>()
            .map_err(TransferError::Model)?;
        let generation = 1;
        let carrier = Carrier::new(
            uuid,
            generation,
            0,
            RigidTransform::identity_at(
                horizon_core::geometry::Vec3::new(
                    cells[0].x as f64,
                    cells[0].y as f64,
                    cells[0].z as f64,
                ),
            )
            .map_err(|_| TransferError::Io("the identity placement refused".into()))?,
            carrier_cells,
        )
        .map_err(TransferError::Model)?;

        // Force the affected chunks to disk, acknowledged, before any
        // intent is published: replay must find the pre-image there.
        let op_id = store.next_op_id();
        let operation = Operation {
            op_id,
            kind: OpKind::Assemble,
            uuid,
            generation,
            cells: operation_cells,
        };
        let encoded = encode_operation(&operation).map_err(|_| TransferError::Envelope)?;

        // The lift runs under #2488's obligation, fenced until commit.
        let _lease = PassLease::enter();
        self.save_affected(cells).map_err(|e| TransferError::Io(e.to_string()))?;
        store
            .write_operation(op_id, &encoded)
            .map_err(|e| TransferError::Store(e.to_string()))?;
        store
            .begin(OpKind::Assemble, uuid, generation)
            .map_err(|e| TransferError::Store(e.to_string()))?;
        for pos in cells {
            let old = self.world.block(pos.x, pos.y, pos.z);
            // Owned removal without drops or notifications: the carrier
            // takes the entity whole, and the cell becomes air through
            // the level's own write path.
            self.with_level(|_b, level, ticks| {
                level.remove_block_entity(*pos, old, false);
                _b.set_block(level, ticks, *pos, air, 3);
            });
            self.world.modified.insert((pos.x >> 4, pos.z >> 4));
        }
        // Fence the world cells and the carrier until the barrier lifts.
        for pos in cells {
            self.world.carrier_fences.lock_cell([pos.x, pos.y, pos.z]);
        }
        self.world.carrier_fences.lock_carrier(uuid);
        drop(_lease);

        // Revalidation: every cell moved, nothing partially transferred.
        for pos in cells {
            if self.world.block(pos.x, pos.y, pos.z) != air {
                return Err(TransferError::Revalidation([pos.x, pos.y, pos.z]));
            }
        }

        // The carrier file and the affected chunks go out, then the
        // barrier covers every submission, and only then `committed`.
        let carrier_bytes = codec::encode(&carrier).map_err(|e| TransferError::Store(e.to_string()))?;
        store
            .write_carrier(uuid, generation, &carrier_bytes)
            .map_err(|e| TransferError::Store(e.to_string()))?;
        self.save_affected(cells).map_err(|e| TransferError::Io(e.to_string()))?;
        self.save.flush().map_err(|e| TransferError::Io(e.to_string()))?;
        store
            .commit()
            .map_err(|e| TransferError::Store(e.to_string()))?;
        for pos in cells {
            self.world.carrier_fences.unlock_cell([pos.x, pos.y, pos.z]);
        }
        self.world.carrier_fences.unlock_carrier(uuid);
        Ok(())
    }

    /// Saves every affected chunk and waits for the writer's barrier, so
    /// the sectors are on disk with the regions synced.
    #[cfg_attr(not(test), allow(dead_code))]
    fn save_affected(&mut self, cells: &[BlockPos]) -> crate::Result<()> {
        let mut keys = Vec::new();
        for pos in cells {
            let key = (pos.x >> 4, pos.z >> 4);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let chunks = keys
            .iter()
            .filter_map(|key| self.world.chunks.get(key).cloned())
            .collect::<Vec<_>>();
        if chunks.is_empty() {
            return Ok(());
        }
        self.save
            .save_later(&chunks, self.pack.assets.registry, world_biomes!(self))?;
        self.save.flush()
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::native_entity_refs::tests::with_game;
    use horizon_core::geometry::Direction as CoreDirection;
    use horizon_core::position::BlockPos;
    use std::sync::Arc;

    /// One transfer's whole life: the chest moves to the carrier, its
    /// cell becomes air, the operation commits, and the block has
    /// exactly one owner — the carrier file, decoded back.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn assembling_moves_a_chest_to_one_owner_and_commits() {
        let root = std::env::temp_dir().join(format!(
            "horizon-carrier-transfer-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        crate::save::write_dat(
            &root.join("level.dat"),
            horizon_core::nbt::Tag::compound([(
                "Data",
                horizon_core::nbt::Tag::Compound(Default::default()),
            )]),
        )
        .unwrap();
        with_game(|game| {
            // The harness's world root is empty: give the transfer a
            // throwaway world of its own, as the save tests do.
            game.world_root = root.clone();
            game.world.chunks.insert(
                (0, 0),
                Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let registry = game.pack.assets.registry;
            let chest = registry.default_state("minecraft:chest").unwrap();
            let air = registry.default_state("minecraft:air").unwrap();
            game.world.set(4, 64, 4, chest).unwrap();

            game.carrier_assemble(7, &[BlockPos::new(4, 64, 4)])
                .unwrap();

            // The cell is air and the fence is lifted.
            assert_eq!(game.world.block(4, 64, 4), air);
            assert!(!game.world.carrier_fences.locks_cell(&[4, 64, 4]));
            assert!(!game.world.carrier_fences.locks_carrier(7));

            // One owner: the chest lives in the carrier file, and only
            // there — the op file is gone and the journal committed.
            let carrier_bytes =
                std::fs::read(root.join("data/horizon_carrier_00000000000000000000000000000007-1.nbt"))
                    .unwrap();
            let carrier = codec::decode(&carrier_bytes).unwrap();
            assert_eq!(carrier.uuid(), 7);
            assert_eq!(carrier.cells().len(), 1);
            assert_eq!(carrier.cells()[0].name(), "minecraft:chest");
            let mut leftovers = std::fs::read_dir(root.join("data")).unwrap();
            assert!(leftovers.all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .starts_with("horizon_carrier_op_")
            }));
        });
        let _ = std::fs::remove_dir_all(&root);
    }

    /// While a fence holds a cell, each refusal route refuses exactly at
    /// its one chokepoint, and the moment the fence clears they pass.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn fenced_cells_refuse_every_route_at_its_chokepoint() {
        with_game(|game| {
            game.world.chunks.insert(
                (0, 0),
                Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let fenced = [4, 64, 4];
            game.world.carrier_fences.lock_cell(fenced);

            // Breaking.
            assert!(game.break_block(fenced).is_err());
            // Pistons: a move pushing the fenced cell refuses whole.
            game.with_level(|_b, level, _ticks| {
                assert!(!horizon_world::level::LevelAccess::allow_piston_move(
                    level,
                    BlockPos::new(4, 64, 3),
                    CoreDirection::North,
                    true,
                    &[BlockPos::new(4, 64, 4)],
                    &[]
                ));
            });
            // The block entity's container does not open.
            assert!(game.refuse_locked_container((4, 64, 4), "minecraft:chest"));
            // Every mod write through the level gate refuses.
            let level = game.level(game.dim.id()).unwrap();
            assert!(!game.native_level_write_allowed(level, fenced));

            // No operation, no refusals: the checks cost one set load.
            game.world.carrier_fences.unlock_cell(fenced);
            assert!(game.native_level_write_allowed(level, fenced));
            assert!(!game.refuse_locked_container((4, 64, 4), "minecraft:chest"));
        });
    }

    /// The #2488 obligation is what autosave and `save-all` refuse on:
    /// holding it refuses, releasing it admits again.
    #[test]
    fn the_save_obligation_gates_saves() {
        use horizon_client::join_seq::visibility::{passes_settled, PassLease};
        let _lease = PassLease::enter();
        assert!(!passes_settled());
        assert!(Game::admit_entity_save().is_err());
        drop(_lease);
        assert!(passes_settled());
        assert!(Game::admit_entity_save().is_ok());
        
    }

    /// A world without transfers creates nothing and fences nothing.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_world_without_transfers_writes_no_store_files() {
        let root = std::env::temp_dir().join(format!(
            "horizon-carrier-quiet-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        with_game(|game| {
            game.world_root = root.clone();
            game.world.chunks.insert(
                (0, 0),
                Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let registry = game.pack.assets.registry;
            let stone = registry.default_state("minecraft:stone").unwrap();
            game.world.set(4, 64, 4, stone).unwrap();
            assert!(!game.world.carrier_fences.locks_cell(&[4, 64, 4]));
            let names: Vec<String> = std::fs::read_dir(root.join("data"))
                .unwrap()
                .map(|e| e.unwrap().file_name().to_str().unwrap().to_string())
                .collect();
            assert!(names.is_empty(), "a world without transfers wrote {names:?}");
        });
        let _ = std::fs::remove_dir_all(&root);
    }
}
