use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// `IItemExtension.onEntityItemUpdate`: a dropped item entity holding
    /// `stack` is about to tick (`ItemEntity.tick`). Set `canceled` and its
    /// movement and merging are skipped for this tick. Server-side.
    pub(in crate::game) fn native_on_entity_item_update(
        &mut self,
        call: mod_api::__private::OnEntityItemUpdateCall<'_>,
    ) {
        self.fire_on_entity_item_update(call);
    }

    /// Asks the parts of the item entity at `index` of `dropped` whether its tick
    /// ends here. An item no part is attached to is told nothing: its stack is not
    /// converted and no handle is made.
    pub(in crate::game) fn item_update_halts(&mut self, index: usize) -> bool {
        let item = &self.dropped[index];
        if !mod_api::__private::item_listens(mod_api::HookId::OnEntityItemUpdate, &item.stack.item)
        {
            return false;
        }
        let id = item.id;
        let Ok(stack) = item.stack.try_into_api() else {
            return false;
        };
        let Some(handle) = self.entity(id).and_then(|entity| self.item_entity(entity)) else {
            return false;
        };
        let mut canceled = false;
        self.native_on_entity_item_update(mod_api::__private::OnEntityItemUpdateCall {
            stack: &stack,
            item: handle,
            canceled: &mut canceled,
        });
        canceled
    }
}

#[cfg(test)]
mod on_entity_item_update_tests {
    use super::*;
    use horizon_client::chunk::Chunk;
    use mod_api::{
        Flow, Hooks, ItemPart, OnEntityItemUpdateEvent, Registrar, RegistrationContext, ServerHost,
    };
    use std::sync::{Arc, Mutex};

    /// An item part that writes the entity's pickup delay to 7 on every update and
    /// records the entities it was told about; `cancels` ends the tick.
    #[derive(Clone)]
    struct Update {
        item: &'static str,
        cancels: bool,
        heard: Arc<Mutex<Vec<u32>>>,
    }

    #[mod_api::horizon_part]
    impl ItemPart for Update {
        fn on_entity_item_update(
            &self,
            host: &mut dyn ServerHost,
            event: &mut OnEntityItemUpdateEvent<'_>,
        ) -> Flow {
            self.heard.lock().unwrap().push(event.item.entity().id());
            assert!(event.item.set_pickup_delay(host, 7));
            if self.cancels {
                Flow::Cancel
            } else {
                Flow::Continue
            }
        }
    }

    struct Mod(Vec<Update>);

    #[mod_api::__subscriptions]
    impl Hooks for Mod {
        fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
            for part in &self.0 {
                let key = r.item_key(part.item);
                r.item_part(key, part.clone());
            }
        }
    }

    /// A serving level with a stone floor at y 63 and nothing dropped.
    fn floor(game: &mut Game<'_>) {
        game.options.dedicated = Some(Vec::new());
        game.native_lifecycle
            .begin_open(mod_api::Authority::IntegratedServer);
        game.world = horizon_client::world::World::new(game.res.facts().air, 1);
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let stone = game
            .pack
            .assets
            .registry
            .default_state("minecraft:stone")
            .unwrap();
        for x in 0..16 {
            for z in 0..16 {
                game.world.set(x, 63, z, stone).unwrap();
            }
        }
        game.dropped.clear();
    }

    /// One stack of `item` at rest on the floor, a pickup delay of 20 ticks.
    fn rest(game: &mut Game<'_>, item: &str, x: f64, z: f64) -> u32 {
        game.spawn_item(ItemStack::new(item, 1), [x, 64.0, z], [0.0; 3], 20);
        game.ticked_entities_through = game.next_entity;
        game.dropped.last().unwrap().id
    }

    fn counts(game: &Game<'_>, ids: &[u32]) -> Vec<u32> {
        game.dropped
            .iter()
            .filter(|item| ids.contains(&item.id))
            .map(|item| item.stack.count)
            .collect()
    }

    /// An item the part of its own item cancels does not age, merge or lose pickup
    /// delay for the tick, and keeps the part's write; an item whose part does not
    /// cancel ticks as vanilla does after the write; an item without a part ticks
    /// and merges as before and is never told.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and an isolated mod registry"]
    fn a_canceling_part_holds_its_own_item_and_nothing_else() {
        crate::game::in_own_process(
            "game::native_hooks::on_entity_item_update::on_entity_item_update_tests::a_canceling_part_holds_its_own_item_and_nothing_else",
            a_canceling_part_holds_its_own_item_and_nothing_else_isolated,
        );
    }

    fn a_canceling_part_holds_its_own_item_and_nothing_else_isolated() {
        use crate::game::native_entity_refs::tests::with_game;
        let iron = Update {
            item: "minecraft:iron_ingot",
            cancels: true,
            heard: Default::default(),
        };
        let gold = Update {
            item: "minecraft:gold_ingot",
            cancels: false,
            heard: Default::default(),
        };
        let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
            "item_update",
            Box::new(Mod(vec![iron.clone(), gold.clone()])),
        )]
        .into();
        mod_api::testing::with_mods(mods.clone(), || {
            crate::game::api_values::install_parts(&mods).unwrap();
            with_game(|game| {
                floor(game);
                // Two of each of iron and diamond side by side, a gold one apart.
                let irons = [
                    rest(game, "minecraft:iron_ingot", 2.5, 2.5),
                    rest(game, "minecraft:iron_ingot", 2.7, 2.5),
                ];
                let diamonds = [
                    rest(game, "minecraft:diamond", 10.5, 2.5),
                    rest(game, "minecraft:diamond", 10.7, 2.5),
                ];
                let gold_id = rest(game, "minecraft:gold_ingot", 6.5, 10.5);

                game.tick_items();
                for id in irons {
                    let held = game.dropped.iter().find(|item| item.id == id).unwrap();
                    // Nothing of the entity's own tick ran: not its age, nor its delay's
                    // countdown; only the part's write is there.
                    assert_eq!((held.age, held.ticks, held.pickup_delay), (0, 0, 7));
                }
                for id in diamonds {
                    let ticked = game.dropped.iter().find(|item| item.id == id).unwrap();
                    assert_eq!((ticked.age, ticked.pickup_delay), (1, 19));
                }
                let gold_item = game.dropped.iter().find(|item| item.id == gold_id).unwrap();
                // The part wrote 7, and then the item's own tick counted down.
                assert_eq!((gold_item.age, gold_item.pickup_delay), (1, 6));

                // A resting item merges within 40 ticks (`ItemEntity.tick`): the part-less
                // pair does, the pair held by its part never does.
                for _ in 0..40 {
                    game.tick_items();
                }
                assert_eq!(counts(game, &irons), [1, 1]);
                assert_eq!(counts(game, &diamonds), [2]);
                assert!(
                    game.dropped
                        .iter()
                        .filter(|item| irons.contains(&item.id))
                        .all(|item| item.age == 0)
                );

                // Told once per tick of each item of its own: 41 ticks, two iron items; 41, one gold item.
                let mut heard = iron.heard.lock().unwrap().clone();
                heard.sort_unstable();
                let mut expected: Vec<u32> = irons.iter().flat_map(|id| [*id; 41]).collect();
                expected.sort_unstable();
                assert_eq!(heard, expected);
                assert_eq!(*gold.heard.lock().unwrap(), vec![gold_id; 41]);
            });
        });
    }

    /// With no part attached to any item, every item ticks and merges as it did before the hook.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn without_parts_items_tick_and_merge() {
        crate::game::native_entity_refs::tests::with_game(|game| {
            floor(game);
            let ids = [
                rest(game, "minecraft:iron_ingot", 2.5, 2.5),
                rest(game, "minecraft:iron_ingot", 2.7, 2.5),
            ];
            game.tick_items();
            assert!(
                game.dropped
                    .iter()
                    .all(|item| (item.age, item.pickup_delay) == (1, 19))
            );
            for _ in 0..40 {
                game.tick_items();
            }
            assert_eq!(counts(game, &ids), [2]);
        });
    }
}
