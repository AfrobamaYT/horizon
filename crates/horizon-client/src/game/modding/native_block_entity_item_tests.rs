use super::api_values::{StackReadError, TryIntoApi};
use super::native_live_holder_tests::{mod_holder, with_mod_parts};
use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer};
use horizon_client::items::ItemStack;
use horizon_core::nbt::Tag;
use mod_api::{
    BlockEntityItemError as Error, Host, HostTransaction, ITEMS, ItemResource, ServerHost,
    Transaction,
};
use std::sync::Arc;

fn commit(host: &mut dyn ServerHost, test: impl FnOnce(&mut dyn ServerHost, &Transaction)) {
    host.transaction(|host, tx| {
        test(host, tx);
        Ok::<_, ()>(())
    })
    .unwrap();
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn internal_slots_preserve_automation_limits_and_nested_transactions() {
    const NAME: &str = "game::native_block_entity_item_tests::internal_slots_preserve_automation_limits_and_nested_transactions";
    with_mod_parts(NAME, |root| {
        super::native_entity_refs::tests::with_game_at(root, |game| {
            let entity = mod_holder(game);
            let get = |host: &dyn Host, slot| entity.item_slot(host, slot);
            let set = |host: &mut dyn ServerHost, tx: &Transaction, slot, stack| {
                entity.set_item_slot(host, tx, slot, stack)
            };
            let pos = (1, 64, 1);
            let stone = game.world.block(1, 64, 1);
            let granite = game
                .pack
                .assets
                .registry
                .default_state("minecraft:granite")
                .unwrap();
            let dirt = ItemStack::new("minecraft:dirt", 2).try_into_api().unwrap();
            let diamond = ItemStack::new("minecraft:diamond", 1)
                .try_into_api()
                .unwrap();
            assert_eq!(get(game, 0), Err(Error::Unavailable));
            commit(game, |host, tx| {
                assert_eq!(
                    set(host, tx, 0, Some(dirt.clone())),
                    Err(Error::Unavailable)
                );
            });
            let machine_slots = ModContainer {
                slots: 2,
                inputs: vec![0],
                outputs: vec![1],
                ..Default::default()
            };
            let replacement_slots = ModContainer {
                slots: 1,
                ..Default::default()
            };
            game.world.block_entities.set_mod_containers(
                [
                    (stone.0, Arc::new(machine_slots)),
                    (granite.0, Arc::new(replacement_slots)),
                ]
                .into_iter()
                .collect(),
            );
            assert_eq!(get(game, 0), Ok(None));
            assert_eq!(
                get(game, 2),
                Err(Error::SlotOutOfBounds { slot: 2, slots: 2 })
            );
            commit(game, |host, tx| {
                assert_eq!(set(host, tx, 1, None), Ok(false));
                assert_eq!(set(host, tx, 0, Some(dirt.clone())), Ok(true));
                let level = entity.level(host).unwrap();
                let external = level.capability(host, &ITEMS, [1, 64, 1], None).unwrap();
                assert_eq!(
                    external.insert(host, tx, 1, &ItemResource::of(&diamond), 1),
                    0
                );
            });
            let BlockEntity::Mod(holder) = game.world.block_entities.get_mut(&pos).unwrap() else {
                panic!()
            };
            holder
                .items
                .push(Some(ItemStack::new("minecraft:gold_ingot", 3)));
            game.world.modified.clear();
            let aborted: Result<(), ()> = (game as &mut dyn ServerHost).transaction(|host, tx| {
                assert_eq!(
                    set(
                        host,
                        tx,
                        0,
                        Some(ItemStack::new("minecraft:dirt", 1).try_into_api().unwrap())
                    ),
                    Ok(true)
                );
                host.transaction(|host, child| {
                    assert_eq!(set(host, child, 1, Some(diamond.clone())), Ok(true));
                    Ok(())
                })?;
                assert_eq!(get(host, 1), Ok(Some(diamond.clone())));
                Err(())
            });
            assert_eq!(aborted, Err(()));
            assert_eq!(get(game, 0), Ok(Some(dirt.clone())));
            assert_eq!(get(game, 1), Ok(None));
            commit(game, |host, tx| {
                assert_eq!(
                    set(
                        host,
                        tx,
                        0,
                        Some(ItemStack::new("minecraft:dirt", 1).try_into_api().unwrap())
                    ),
                    Ok(true)
                );
                assert_eq!(set(host, tx, 1, Some(diamond.clone())), Ok(true));
            });
            assert_eq!(
                get(game, 0),
                Ok(Some(
                    ItemStack::new("minecraft:dirt", 1).try_into_api().unwrap()
                ))
            );
            assert_eq!(get(game, 1), Ok(Some(diamond.clone())));
            assert!(!game.world.modified.is_empty());
            let BlockEntity::Mod(holder) = game.world.block_entities.get(&pos).unwrap() else {
                panic!()
            };
            assert_eq!(
                holder.items[2],
                Some(ItemStack::new("minecraft:gold_ingot", 3)),
                "saved excess is not truncated"
            );
            let before_revision = holder.revision();
            assert_eq!(
                ItemStack::new("missing:item", 1).try_into_api(),
                Err(StackReadError::UnknownItem)
            );
            assert_eq!(
                ItemStack::new("minecraft:dirt", 0).try_into_api(),
                Err(StackReadError::Empty)
            );
            game.world.modified.clear();
            let mut component = ItemStack::new("minecraft:diamond", 5);
            component
                .data_mut()
                .set_other("minecraft:max_stack_size", Some(Tag::Int(4)));
            commit(game, |host, tx| {
                assert_eq!(set(host, tx, 1, Some(diamond.clone())), Ok(false));
                assert_eq!(
                    set(host, tx, 2, None),
                    Err(Error::SlotOutOfBounds { slot: 2, slots: 2 })
                );
                for (stack, error) in [
                    (
                        ItemStack::new("minecraft:diamond", 65),
                        Error::StackRejected(mod_api::StackWriteError::CountExceeded {
                            maximum: 64,
                        }),
                    ),
                    (
                        ItemStack::new("minecraft:diamond_sword", 2),
                        Error::StackRejected(mod_api::StackWriteError::CountExceeded {
                            maximum: 1,
                        }),
                    ),
                    (
                        component.clone(),
                        Error::StackRejected(mod_api::StackWriteError::CountExceeded {
                            maximum: 4,
                        }),
                    ),
                    (ItemStack::new("minecraft:air", 1), Error::InvalidStack),
                ] {
                    let stack = stack.try_into_api().unwrap();
                    assert_eq!(set(host, tx, 1, Some(stack)), Err(error));
                }
            });
            let BlockEntity::Mod(holder) = game.world.block_entities.get(&pos).unwrap() else {
                panic!()
            };
            assert_eq!(holder.revision(), before_revision);
            assert_eq!(get(game, 1), Ok(Some(diamond)));
            assert!(game.world.modified.is_empty());
            component.count = 4;
            commit(game, |host, tx| {
                assert_eq!(
                    set(host, tx, 1, Some(component.try_into_api().unwrap())),
                    Ok(true)
                );
                assert_eq!(get(host, 1), Ok(Some(component.try_into_api().unwrap())));
                assert_eq!(set(host, tx, 1, None), Ok(true));
            });
            assert_eq!(get(game, 1), Ok(None));
            let level = entity.level(game).unwrap();
            game.world.block_entities.remove(&pos);
            assert_eq!(get(game, 0), Err(Error::Stale));
            commit(game, |host, tx| {
                assert_eq!(set(host, tx, 0, None), Err(Error::Stale));
            });
            game.world
                .block_entities
                .insert(pos, BlockEntity::Chest(vec![None; 27]));
            commit(game, |host, tx| {
                assert_eq!(get(host, 0), Err(Error::Stale));
                assert_eq!(set(host, tx, 0, None), Err(Error::Stale));
            });
            game.world.set(1, 64, 1, granite).unwrap();
            let replacement = ModBlockEntity::new("live_bench:ticker".into(), Default::default());
            game.world
                .block_entities
                .insert(pos, BlockEntity::Mod(Box::new(replacement)));
            assert_eq!(get(game, 0), Err(Error::Stale));
            let replacement = game.block_entity(level, [1, 64, 1]).unwrap();
            assert_ne!(entity, replacement);
            let get = |host: &dyn Host, slot| replacement.item_slot(host, slot);
            let set = |host: &mut dyn ServerHost, tx: &Transaction, slot, stack| {
                replacement.set_item_slot(host, tx, slot, stack)
            };
            commit(game, |host, tx| {
                assert_eq!(
                    set(host, tx, 1, None),
                    Err(Error::SlotOutOfBounds { slot: 1, slots: 1 })
                );
                assert_eq!(set(host, tx, 0, Some(dirt.clone())), Ok(true));
            });
            assert_eq!(get(game, 0), Ok(Some(dirt)));
            game.native_lifecycle
                .begin_open(mod_api::Authority::IntegratedServer);
            assert_eq!(get(game, 0), Err(Error::Stale));
            assert!(game.transactions.is_empty());
        });
    });
}

/// A mod holder with nine container slots and `keys` entries of saved data beside them.
fn container_holder(game: &mut super::Game<'_>, keys: usize) -> mod_api::BlockEntityRef {
    let entity = mod_holder(game);
    let stone = game.world.block(1, 64, 1);
    game.world.block_entities.set_mod_containers(
        [(
            stone.0,
            Arc::new(ModContainer {
                slots: 9,
                inputs: (0..9).collect(),
                outputs: (0..9).collect(),
                ..Default::default()
            }),
        )]
        .into_iter()
        .collect(),
    );
    let BlockEntity::Mod(holder) = game.world.block_entities.get_mut(&(1, 64, 1)).unwrap() else {
        panic!()
    };
    holder.items = vec![None; 9];
    for key in 0..keys {
        holder.nbt.insert(format!("k{key}").as_str().into(), Tag::Long(key as i64));
    }
    entity
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn container_moves_roll_back_slots_and_saved_data_in_any_order() {
    const NAME: &str = "game::native_block_entity_item_tests::container_moves_roll_back_slots_and_saved_data_in_any_order";
    with_mod_parts(NAME, |root| {
        super::native_entity_refs::tests::with_game_at(root, |game| {
            let entity = container_holder(game, 3);
            let level = entity.level(game).unwrap();
            let items = level.capability(game, &ITEMS, [1, 64, 1], None).unwrap();
            let dirt = ItemResource::of(
                &ItemStack::new("minecraft:dirt", 1)
                    .try_into_api()
                    .unwrap(),
            );
            let saved = |game: &super::Game<'_>| {
                let BlockEntity::Mod(holder) = game.world.block_entities.get(&(1, 64, 1)).unwrap()
                else {
                    panic!()
                };
                (holder.items.clone(), holder.nbt.clone(), holder.data.clone())
            };
            commit(game, |host, tx| assert_eq!(items.insert(host, tx, 0, &dirt, 10), 10));
            let before = saved(game);
            // Slot writes, then a whole-entity write, then slots again, through nested
            // transactions that commit into one that aborts.
            let aborted: Result<(), ()> = (game as &mut dyn ServerHost).transaction(|host, tx| {
                assert_eq!(items.extract(host, tx, 0, &dirt, 4), 4);
                assert_eq!(items.insert(host, tx, 3, &dirt, 7), 7);
                host.transaction(|host, child| {
                    assert_eq!(items.insert(host, child, 0, &dirt, 50), 50);
                    assert_eq!(entity.set_data_slots(host, vec![7]), Ok(true));
                    assert_eq!(items.extract(host, child, 3, &dirt, 2), 2);
                    Ok::<_, ()>(())
                })?;
                host.transaction(|host, child| {
                    assert_eq!(items.insert(host, child, 8, &dirt, 1), 1);
                    Err::<(), ()>(())
                })
                .unwrap_err();
                assert_eq!(items.amount(host, 8), 0, "the child's abort undid its slot");
                assert_eq!(items.amount(host, 0), 56);
                Err(())
            });
            assert_eq!(aborted, Err(()));
            assert_eq!(saved(game), before, "slots, saved data and data slots back as they were");
            // The same writes committed stay.
            commit(game, |host, tx| {
                assert_eq!(items.extract(host, tx, 0, &dirt, 4), 4);
                assert_eq!(entity.set_data_slots(host, vec![9]), Ok(true));
                assert_eq!(items.insert(host, tx, 5, &dirt, 3), 3);
            });
            let (slots, nbt, data) = saved(game);
            assert_eq!(slots[0].as_ref().map(|s| s.count), Some(6));
            assert_eq!(slots[5].as_ref().map(|s| s.count), Some(3));
            assert_eq!(data, [9]);
            assert_eq!(nbt.len(), 3, "the saved data beside the slots is untouched");
            assert!(game.transactions.is_empty());
        });
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn a_container_move_costs_the_same_whatever_saved_data_sits_beside_the_slots() {
    const NAME: &str = "game::native_block_entity_item_tests::a_container_move_costs_the_same_whatever_saved_data_sits_beside_the_slots";
    with_mod_parts(NAME, |root| {
        super::native_entity_refs::tests::with_game_at(root, |game| {
            let dirt = ItemResource::of(
                &ItemStack::new("minecraft:dirt", 1)
                    .try_into_api()
                    .unwrap(),
            );
            // Moves as `move_once` makes them: each in its own transaction, nested in a root.
            let time = |game: &mut super::Game<'_>, keys: usize| {
                let entity = container_holder(game, keys);
                let level = entity.level(game).unwrap();
                let items = level.capability(game, &ITEMS, [1, 64, 1], None).unwrap();
                let start = std::time::Instant::now();
                commit(game, |host, _| {
                    for _ in 0..500 {
                        host.transaction(|host, tx| {
                            assert_eq!(items.insert(host, tx, 0, &dirt, 1), 1);
                            Ok::<_, ()>(())
                        })
                        .unwrap();
                        host.transaction(|host, tx| {
                            assert_eq!(items.extract(host, tx, 0, &dirt, 1), 1);
                            Ok::<_, ()>(())
                        })
                        .unwrap();
                    }
                });
                start.elapsed()
            };
            time(game, 1);
            let small = time(game, 1);
            let large = time(game, 100_000);
            eprintln!("1,000 container moves: {small:?} beside 1 key, {large:?} beside 100,000");
            assert!(
                large < small * 4 + std::time::Duration::from_millis(20),
                "a move copied the saved data: {small:?} beside 1 key, {large:?} beside 100,000"
            );
        });
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn storage_asks_a_game_container_by_kind_and_rolls_back_with_its_transaction() {
    const NAME: &str = "game::native_block_entity_item_tests::storage_asks_a_game_container_by_kind_and_rolls_back_with_its_transaction";
    with_mod_parts(NAME, |root| {
        super::native_entity_refs::tests::with_game_at(root, |game| {
            let entity = mod_holder(game);
            let level = entity.level(game).unwrap();
            let chest = game
                .pack
                .assets
                .registry
                .default_state("minecraft:chest")
                .unwrap();
            game.world.set(3, 64, 1, chest).unwrap();
            game.world
                .block_entities
                .insert((3, 64, 1), BlockEntity::Chest(vec![None; 27]));
            let of = |id: &str| {
                ItemResource::of(&ItemStack::new(id, 1).try_into_api().unwrap())
            };
            let (stone, dirt) = (of("minecraft:stone"), of("minecraft:dirt"));
            let storage = level
                .capability(game, &mod_api::STORAGE, [3, 64, 1], None)
                .unwrap();
            assert_eq!(storage.space_for(game, &stone), 27 * 64);
            commit(game, |host, tx| {
                assert_eq!(storage.insert(host, tx, &stone, 100), 100);
                assert_eq!(storage.insert(host, tx, &dirt, 5), 5);
            });
            assert_eq!(storage.amount(game, &stone), 100);
            assert_eq!(storage.space_for(game, &stone), 24 * 64 + 28, "two stone slots, one dirt");
            let aborted: Result<(), ()> = (game as &mut dyn ServerHost).transaction(|host, tx| {
                assert_eq!(storage.extract(host, tx, &stone, 70), 70);
                assert_eq!(storage.insert(host, tx, &dirt, 2000), 25 * 64 + 59);
                Err(())
            });
            assert!(aborted.is_err());
            assert_eq!(storage.amount(game, &stone), 100);
            assert_eq!(storage.amount(game, &dirt), 5);
            let BlockEntity::Chest(slots) = game.world.block_entities.get(&(3, 64, 1)).unwrap() else {
                panic!()
            };
            let counts: Vec<u32> = slots.iter().flatten().map(|s| s.count).collect();
            assert_eq!(counts, [64, 36, 5], "slots back as they were, in their order");
            let mut cursor = 0;
            let mut listed = Vec::new();
            assert!(storage.page(game, &mut cursor, 64, &mut listed));
            assert_eq!(listed.len(), 3);
            // A furnace's top takes only into its input, and its bottom gives only its result.
            let furnace = game
                .pack
                .assets
                .registry
                .default_state("minecraft:furnace")
                .unwrap();
            game.world.set(5, 64, 1, furnace).unwrap();
            game.world.block_entities.insert(
                (5, 64, 1),
                BlockEntity::for_block("minecraft:furnace").unwrap(),
            );
            let top = level
                .capability(game, &mod_api::STORAGE, [5, 64, 1], Some(mod_api::Direction::Up))
                .unwrap();
            let bottom = level
                .capability(game, &mod_api::STORAGE, [5, 64, 1], Some(mod_api::Direction::Down))
                .unwrap();
            commit(game, |host, tx| {
                assert_eq!(top.insert(host, tx, &stone, 200), 64, "the input slot only");
                assert_eq!(bottom.extract(host, tx, &stone, 64), 0, "not the input");
            });
            assert!(game.transactions.is_empty());
        });
    });
}
