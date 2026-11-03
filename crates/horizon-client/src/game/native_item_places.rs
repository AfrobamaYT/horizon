//! The game's item places for native mods (`mod_api::ItemPlace`, arismoko/horizon#41 §7): a
//! player's slots, hands and cursor, entities' slots and item entities. Mods read and write
//! them through `ItemPlace`'s `stack`, `insert` and `extract`, which come here for these places; a block entity's slots,
//! a stack inside an item and a mod's own places go through their handlers.
use super::transactions::After;
use super::{Game, minecarts::CartKind};
use horizon_client::{
    first_person::Hand,
    items::ItemStack,
    mod_api::{ItemPlace, Transaction},
};

/// The player a place belongs to, if a player's.
fn place_player(place: &ItemPlace) -> Option<u128> {
    match place {
        ItemPlace::PlayerSlot { player, .. }
        | ItemPlace::PlayerHand { player, .. }
        | ItemPlace::PlayerCursor { player } => Some(*player),
        _ => None,
    }
}

impl Game<'_> {
    /// The inventory of entity `id` in the level in place, if it has one: a chest or hopper
    /// minecart's (the open menu's copy while one shows it, as `cart_slots`), a chest boat's, or
    /// a chested donkey's, mule's or llama's (`AbstractHorse.inventory`, `getInventoryColumns`
    /// rows of three). With where it stands, for what a split throws.
    pub(super) fn entity_slots(&mut self, id: u32) -> Option<(&mut [Option<ItemStack>], [f64; 3])> {
        let key = (i32::MAX - 15, super::minecarts::CART_KEY_Y, id as i32);
        let at = |p: &horizon_core::geometry::Vec3| [p.x, p.y, p.z];
        let container = |c: &super::minecarts::Minecart| {
            c.id == id && matches!(c.kind, CartKind::Chest | CartKind::Hopper)
        };
        let position = if let Some(c) = self.carts.iter().find(|c| container(c)) {
            at(&c.position)
        } else if let Some(b) = self.boats.iter().find(|b| b.id == id && b.chest) {
            at(&b.position)
        } else {
            let m = self
                .mobs
                .iter_mut()
                .find(|m| m.id == id && m.extra.chested)?;
            let size = super::mount_menu::mount_columns(m) * 3;
            if m.extra.items.len() < size {
                if let Some(level) = self.transactions.last_mut() {
                    level.keep_mob_length(id, m.extra.items.len());
                }
                m.extra.items.resize(size, None);
            }
            return Some((&mut m.extra.items[..size], at(&m.position)));
        };
        if self.carts.iter().any(container) {
            return Some((self.cart_slots(id)?, position));
        }
        if let Some(super::BlockEntity::Chest(open)) = self.world.block_entities.get_mut(&key) {
            return Some((open, position));
        }
        let b = self.boats.iter_mut().find(|b| b.id == id)?;
        Some((&mut b.items, position))
    }
    /// Whether this game simulates a world: a client's copy has no places to write.
    fn places_simulated(&self) -> bool {
        self.serving.is_some() || self.options.dedicated.is_some()
    }
    /// The slot of the player in place that `place` names.
    fn player_place(&mut self, place: &ItemPlace) -> Option<&mut Option<ItemStack>> {
        match place {
            ItemPlace::PlayerSlot { slot, .. } => self.native_carried_slot(*slot),
            ItemPlace::PlayerHand {
                hand: Hand::Main, ..
            } => self.inventory.get_mut(self.selected),
            ItemPlace::PlayerHand {
                hand: Hand::Off, ..
            } => Some(&mut self.offhand),
            ItemPlace::PlayerCursor { .. } => Some(&mut self.carried),
            _ => None,
        }
    }
    /// How many slots the inventory of entity `entity` in the level in place has, as
    /// `entity_slots` reaches them: none for an entity without one, and on a client.
    pub(super) fn native_entity_container_size(&self, entity: u32) -> Option<usize> {
        if self.remote.is_some() {
            return None;
        }
        if let Some(c) = self.carts.iter().find(|c| c.id == entity) {
            let container = matches!(c.kind, CartKind::Chest | CartKind::Hopper);
            return container.then_some(c.items.len());
        }
        if let Some(b) = self.boats.iter().find(|b| b.id == entity) {
            return b.chest.then_some(b.items.len());
        }
        let m = self
            .mobs
            .iter()
            .find(|m| m.id == entity && m.extra.chested)?;
        Some(super::mount_menu::mount_columns(m) * 3)
    }
    /// `EntityType.is(TagKey)`: whether `entity_type` (`minecraft:player`) is in the entity
    /// type tag `tag` (`minecraft:cannot_be_pushed_onto_boats`). API @API:item_places@.
    pub(super) fn native_entity_type_in_tag(&self, entity_type: &str, tag: &str) -> bool {
        let tag = horizon_client::command::args::normalize(tag);
        self.res.mob_data().entity_has(&tag, entity_type)
    }
    /// The stack at a player's or an item entity's place; none for an empty one, a player who
    /// isn't here or is dead, an item entity not in the level in place, any other place
    /// (`ItemPlace::stack` reads those), and on a client.
    pub(super) fn native_game_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        if !self.places_simulated() {
            return None;
        }
        if let ItemPlace::ItemEntity { id } = place {
            return self
                .dropped
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.stack.clone());
        }
        if let ItemPlace::EntitySlot { entity, slot } = place {
            let (slots, _) = self.entity_slots(*entity)?;
            return slots.get(*slot).cloned().flatten().filter(|s| s.count > 0);
        }
        let mut stack = None;
        self.native_as_player(place_player(place)?, |g| {
            stack = g.player_place(place).and_then(|slot| slot.clone());
        });
        stack.filter(|s| s.count > 0)
    }
    /// `ItemAccess.insert` at a player's place, an item entity's or an entity's slot: up to
    /// `stack.count` of `stack`, into the place where it's empty or holds the same item and
    /// components, up to the item's stack size. The rest goes where vanilla puts an item split
    /// off it: a player's into their inventory, else dropped as `Player.drop(stack, false)` at
    /// the root commit (a creative player's hand takes anything, and gives them one if they have
    /// none, as `ItemUtils.createFilledResult`); an item entity's dropped beside it at the root
    /// commit; an entity slot's into the entity's other slots (`forHandlerIndex`). How much
    /// went: 0 where `native_game_stack` reads nothing for want of the place. In transaction
    /// `tx`, which keeps each place as it was. The player's screen follows with the tick's sync.
    pub(super) fn native_game_insert(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        tx: &Transaction,
    ) -> u32 {
        let _ = tx;
        if !self.places_simulated() || stack.count == 0 {
            return 0;
        }
        let max = self.containers().max_stack(&stack);
        // Puts what of `rest` fits into `slot`.
        let fill = |slot: &mut Option<ItemStack>, rest: &mut ItemStack| {
            let held = slot.as_ref().map_or(0, |held| held.count);
            let room = match slot {
                Some(h) if held > 0 && !h.same_item_same_components(rest) => 0,
                _ => max.saturating_sub(held),
            };
            let put = room.min(rest.count);
            if put > 0 {
                *slot = Some(ItemStack {
                    count: held + put,
                    ..rest.clone()
                });
                rest.count -= put;
            }
        };
        let mut rest = stack.clone();
        if let ItemPlace::ItemEntity { id } = place {
            let Some(e) = self.dropped.iter().find(|e| e.id == *id) else {
                return 0;
            };
            let at = [e.position.x, e.position.y, e.position.z];
            let mut held = Some(e.stack.clone());
            fill(&mut held, &mut rest);
            if let Some(held) = held.filter(|_| rest.count < stack.count) {
                self.journal_set_item_entity(*id, held);
            }
            if rest.count > 0 {
                self.after_commit(After::Spawn(at, rest));
            }
            return stack.count;
        }
        if let ItemPlace::EntitySlot { entity, slot } = place {
            if !self
                .entity_slots(*entity)
                .is_some_and(|(slots, _)| *slot < slots.len())
            {
                return 0;
            }
            self.journal_entity(*entity);
            let Some((slots, _)) = self.entity_slots(*entity) else {
                return 0;
            };
            fill(&mut slots[*slot], &mut rest);
            for (i, held) in slots.iter_mut().enumerate() {
                if i != *slot {
                    fill(held, &mut rest);
                }
            }
            return stack.count - rest.count;
        }
        let Some(player) = place_player(place) else {
            return 0;
        };
        let mut went = 0;
        self.native_as_player(player, |g| {
            if g.player_place(place).is_none() {
                return;
            }
            went = stack.count;
            g.journal_player(player);
            if g.creative && matches!(place, ItemPlace::PlayerHand { .. }) {
                let have = g.inventory.iter().flatten();
                if !have.clone().any(|h| h.same_item_same_components(&stack)) {
                    g.give(ItemStack { count: 1, ..stack });
                }
                return;
            }
            if let Some(slot) = g.player_place(place) {
                fill(slot, &mut rest);
            }
            if let Some(rest) = (rest.count > 0).then(|| g.give(rest)).flatten() {
                g.drop_after_commit(player, rest);
            }
        });
        went
    }
    /// `ItemAccess.extract` at those places: up to `stack.count` of the item and components of
    /// `stack`, how many came out. An item entity emptied stays, empty, until the root commit
    /// removes it. A creative player's hand gives without losing any (`forInfiniteMaterials`).
    pub(super) fn native_game_extract(
        &mut self,
        place: &ItemPlace,
        stack: ItemStack,
        tx: &Transaction,
    ) -> u32 {
        let _ = tx;
        if !self.places_simulated() {
            return 0;
        }
        // How many of `stack` `held` gives.
        let taken = |held: Option<&ItemStack>| {
            held.filter(|held| held.same_item_same_components(&stack))
                .map_or(0, |held| held.count.min(stack.count))
        };
        let left = |held: &ItemStack, took: u32| ItemStack {
            count: held.count - took,
            ..held.clone()
        };
        if let ItemPlace::ItemEntity { id } = place {
            let Some(e) = self.dropped.iter().find(|e| e.id == *id) else {
                return 0;
            };
            let took = taken(Some(&e.stack));
            let kept = left(&e.stack, took);
            if took > 0 {
                self.journal_set_item_entity(*id, kept);
            }
            return took;
        }
        if let ItemPlace::EntitySlot { entity, slot } = place {
            let Some((slots, _)) = self.entity_slots(*entity) else {
                return 0;
            };
            let took = taken(slots.get(*slot).and_then(Option::as_ref));
            if took == 0 {
                return 0;
            }
            self.journal_entity(*entity);
            let Some((slots, _)) = self.entity_slots(*entity) else {
                return 0;
            };
            let kept = slots[*slot].as_ref().map(|held| left(held, took));
            slots[*slot] = kept.filter(|kept| kept.count > 0);
            return took;
        }
        let Some(player) = place_player(place) else {
            return 0;
        };
        let mut took = 0;
        self.native_as_player(player, |g| {
            let creative = g.creative && matches!(place, ItemPlace::PlayerHand { .. });
            let Some(slot) = g.player_place(place) else {
                return;
            };
            took = taken(slot.as_ref());
            if took == 0 || creative {
                return;
            }
            g.journal_player(player);
            if let Some(slot) = g.player_place(place) {
                let kept = slot.as_ref().map(|held| left(held, took));
                *slot = kept.filter(|kept| kept.count > 0);
            }
        });
        took
    }
    /// Drops `stack` at the middle of the block at `pos`, as a block entity's handler gives up
    /// what it can't put back.
    pub(super) fn native_drop_at(&mut self, pos: [i32; 3], stack: ItemStack) {
        if self.places_simulated() && stack.count > 0 {
            // `Block.popResource`'s default pickup delay, at the root commit.
            self.after_commit(After::Drop(pos, stack));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use horizon_client::{
        items::ItemStack,
        lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig},
        mod_api::{ENTITY_ITEMS, Host, ItemPlace, ItemResource},
    };
    use horizon_protocol::packets::configuration::ClientInformation;
    use std::sync::{Arc, atomic::AtomicI32, mpsc};

    /// A world open to LAN on a dedicated server, as a guest's actions run.
    fn open_to_lan(game: &mut super::Game<'_>) {
        game.options.dedicated = Some(Vec::new());
        let settings = super::super::lan::LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        };
        let net = NetConfig {
            announce: false,
            ..NetConfig::lan(0, "PlaceTest".into(), 2)
        };
        let config = HubConfig {
            default_game_mode: 0,
            guest_commands: false,
            force_game_mode: false,
            hardcore: false,
            view_distance: 10,
            simulation_distance: 10,
            dimension: "minecraft:overworld".into(),
            allow_flight: false,
            seed: 0,
            max_players: 2,
            integrated: false,
            levels: Default::default(),
        };
        game.lan_host(settings, net, config).unwrap();
    }

    /// A placed guest on connection `conn`, with UUID and entity id `conn + 122` and
    /// `conn + 19`.
    fn guest(conn: u32) -> Guest {
        let (tx, _rx) = mpsc::channel();
        let mut guest = Guest::new(
            Joined {
                conn,
                uuid: u128::from(conn) + 122,
                name: format!("guest{conn}"),
                properties: Vec::new(),
                info: ClientInformation::new(8),
                brand: String::new(),
                sender: PlayerSender::new(tx, Arc::default()),
                latency: Arc::new(AtomicI32::new(0)),
                address: "127.0.0.1:1".parse().unwrap(),
            },
            conn as i32 + 19,
            [0.0; 3],
            8,
        );
        guest.placed = true;
        guest
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_bucket_fills_and_empties_whole_through_its_place_and_splits_off_its_stack() {
        use horizon_client::chunk::Chunk;
        use horizon_client::mod_api::{
            FLUIDS, ITEM_FLUIDS, NativeBlockUpdateFlags, move_resources,
        };
        with_game(|game| {
            open_to_lan(game);
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let level = game.native_level("minecraft:overworld").unwrap();
            let cauldron = |game: &super::Game<'_>, n: u8| {
                let registry = &game.pack.assets.registry;
                if n == 0 {
                    return registry.default_state("minecraft:cauldron").unwrap();
                }
                let state = horizon_world::chunk::BlockState {
                    name: "minecraft:water_cauldron".to_owned(),
                    properties: [("level".to_owned(), n.to_string())].into_iter().collect(),
                };
                registry.resolve(&state).unwrap()
            };
            let set = |game: &mut super::Game<'_>, pos, n| {
                let state = cauldron(game, n);
                let flags = NativeBlockUpdateFlags::NEIGHBORS;
                assert!(game.native_level_set_block(level, pos, state, flags));
            };
            let (a, b) = ([0, 64, 0], [2, 64, 0]);
            set(game, a, 3);
            set(game, b, 2);
            let mut player = guest(1);
            player.inventory[0] = Some(ItemStack::new("minecraft:bucket", 3));
            player.inventory[5] = Some(ItemStack::new("minecraft:lava_bucket", 1));
            game.lan.as_mut().unwrap().hub.guests.push(player);
            let slot = |slot| ItemPlace::PlayerSlot { player: 123, slot };
            // Up to a bucket from the cauldron at `at` into the bucket in `slot`, or back.
            let moved = |game: &mut super::Game<'_>, at, slot: &ItemPlace, into_bucket| {
                let host: &mut dyn Host = game;
                let block = level.capability(host, &FLUIDS, at, None).unwrap();
                let bucket = slot.capability(host, &ITEM_FLUIDS).unwrap();
                let (from, to) = match into_bucket {
                    true => (&block, &bucket),
                    false => (&bucket, &block),
                };
                host.transaction(|host, tx| move_resources(host, from, to, |_| true, 1000, tx))
            };
            let inventory =
                |game: &super::Game<'_>| game.lan.as_ref().unwrap().hub.guests[0].inventory.clone();
            let block =
                |game: &super::Game<'_>, pos: [i32; 3]| game.world.block(pos[0], pos[1], pos[2]);
            // A full cauldron fills one of the three buckets, which splits off.
            assert_eq!(moved(game, a, &slot(0), true), Ok(1000));
            assert_eq!(block(game, a), cauldron(game, 0));
            let held = inventory(game);
            assert_eq!(held[0], Some(ItemStack::new("minecraft:bucket", 2)));
            let water = ItemStack::new("minecraft:water_bucket", 1);
            let filled = held
                .iter()
                .position(|s| s.as_ref() == Some(&water))
                .unwrap();
            // Two thirds of a bucket fill none: a bucket takes a whole one or nothing.
            assert_eq!(moved(game, b, &slot(0), true), Ok(0));
            assert_eq!(block(game, b), cauldron(game, 2));
            assert_eq!(inventory(game), held);
            // The water bucket empties into the empty cauldron and is a bucket again.
            let filled = slot(i32::try_from(filled).unwrap());
            assert_eq!(moved(game, a, &filled, false), Ok(1000));
            assert_eq!(block(game, a), cauldron(game, 3));
            assert_eq!(
                filled.stack(game),
                Some(ItemStack::new("minecraft:bucket", 1))
            );
            // Lava doesn't go into a water cauldron, and the lava bucket stays full.
            assert_eq!(moved(game, a, &slot(5), false), Ok(0));
            let lava = Some(ItemStack::new("minecraft:lava_bucket", 1));
            assert_eq!(inventory(game)[5], lava);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_guests_place_is_written_as_theirs_and_an_abort_puts_it_back() {
        with_game(|game| {
            open_to_lan(game);
            let mut guest = guest(1);
            guest.inventory[0] = Some(ItemStack::new("minecraft:bucket", 3));
            game.lan.as_mut().unwrap().hub.guests.push(guest);
            let host_before = game.inventory.clone();
            let guest_inventory =
                |game: &super::Game<'_>| game.lan.as_ref().unwrap().hub.guests[0].inventory.clone();
            let before = guest_inventory(game);
            let slot = ItemPlace::PlayerSlot {
                player: 123,
                slot: 0,
            };
            let water = ItemResource::of(&ItemStack::new("minecraft:water_bucket", 1));

            // Aborted, the guest's inventory is as it was, and the host's was never touched.
            let host: &mut dyn Host = game;
            let _ = host.transaction(|host, tx| {
                assert_eq!(slot.exchange(host, tx, &water, 1), 1);
                assert_eq!(slot.amount(host), 2);
                Err::<(), ()>(())
            });
            assert_eq!(guest_inventory(game), before);
            assert_eq!(game.inventory, host_before);

            // Committed, the guest keeps two buckets and gets the filled one.
            let host: &mut dyn Host = game;
            let _ = host.transaction(|host, tx| Ok::<_, ()>(slot.exchange(host, tx, &water, 1)));
            let after = guest_inventory(game);
            assert_eq!(after[0], Some(ItemStack::new("minecraft:bucket", 2)));
            let filled = after.iter().flatten();
            assert_eq!(
                filled
                    .filter(|s| &*s.item == "minecraft:water_bucket")
                    .count(),
                1
            );
            assert_eq!(game.inventory, host_before);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn an_armour_slot_holds_one() {
        with_game(|game| {
            open_to_lan(game);
            game.lan.as_mut().unwrap().hub.guests.push(guest(1));
            let host: &mut dyn Host = game;
            let items = host
                .entity(20)
                .unwrap()
                .capability(host, &ENTITY_ITEMS)
                .unwrap();
            let pumpkin = ItemResource::of(&ItemStack::new("minecraft:carved_pumpkin", 1));
            let wrote =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert(host, tx, 39, &pumpkin, 64)));
            assert_eq!(wrote, Ok(1));
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_creative_players_selected_stack_takes_what_fits_and_keeps_it() {
        with_game(|game| {
            open_to_lan(game);
            let mut player = guest(1);
            player.game_mode = 1;
            player.inventory[0] = Some(ItemStack::new("minecraft:stick", 60));
            game.lan.as_mut().unwrap().hub.guests.push(player);
            let host: &mut dyn Host = game;
            let items = host
                .entity(20)
                .unwrap()
                .capability(host, &ENTITY_ITEMS)
                .unwrap();
            let sticks = ItemResource::of(&ItemStack::new("minecraft:stick", 1));
            assert_eq!(
                host.transaction(|host, tx| Ok::<_, ()>(items.insert_any(host, tx, &sticks, 4))),
                Ok(4)
            );
            assert_eq!(
                game.lan.as_ref().unwrap().hub.guests[0].inventory[0]
                    .as_ref()
                    .unwrap()
                    .count,
                64
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_kept_handler_reaches_nothing_once_its_cart_is_removed() {
        use super::super::{
            commands::Target,
            minecarts::{CartKind, Minecart},
        };
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            game.carts.push(Minecart::new(
                500,
                CartKind::Chest,
                horizon_core::geometry::Vec3::ZERO,
            ));
            let host: &mut dyn Host = game;
            let entity = host.entity(500).unwrap();
            let items = entity.capability(host, &ENTITY_ITEMS).unwrap();
            game.persist.removals.push(Target::Cart(500));
            assert!(entity.capability(game, &ENTITY_ITEMS).is_none());
            let sticks = ItemResource::of(&ItemStack::new("minecraft:stick", 1));
            let host: &mut dyn Host = game;
            let wrote =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert(host, tx, 0, &sticks, 1)));
            assert_eq!(wrote, Ok(0));
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn bound_armour_comes_off_only_in_creative() {
        with_game(|game| {
            open_to_lan(game);
            let mut player = guest(1);
            let mut helmet = ItemStack::new("minecraft:iron_helmet", 1);
            helmet
                .data_mut()
                .enchantments
                .push(("minecraft:binding_curse".into(), 1));
            player.armor[0] = Some(helmet.clone());
            game.lan.as_mut().unwrap().hub.guests.push(player);
            let host: &mut dyn Host = game;
            let items = host
                .entity(20)
                .unwrap()
                .capability(host, &ENTITY_ITEMS)
                .unwrap();
            let helmet = ItemResource::of(&helmet);
            assert_eq!(items.amount(host, 39), 1);
            let take = |host: &mut dyn Host| {
                host.transaction(|host, tx| Ok::<_, ()>(items.extract(host, tx, 39, &helmet, 1)))
            };
            assert_eq!(take(host), Ok(0));
            // A creative player takes it off (`ArmorSlot.mayPickup`'s `isCreative`).
            game.lan.as_mut().unwrap().hub.guests[0].game_mode = 1;
            assert_eq!(take(game), Ok(1));
            assert_eq!(game.lan.as_ref().unwrap().hub.guests[0].armor[0], None);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn an_equippable_whose_entity_tag_holds_the_player_fits_its_slot() {
        use horizon_client::mod_api::{ComponentId, Tag};
        with_game(|game| {
            open_to_lan(game);
            game.lan.as_mut().unwrap().hub.guests.push(guest(1));
            let host: &mut dyn Host = game;
            let items = host
                .entity(20)
                .unwrap()
                .capability(host, &ENTITY_ITEMS)
                .unwrap();
            let helmet = ItemResource::of(&ItemStack::new("minecraft:iron_helmet", 1))
                .with_component(
                    host,
                    &ComponentId::new("minecraft:equippable").unwrap(),
                    Tag::Compound(
                        [
                            ("slot".into(), Tag::String("head".into())),
                            (
                                "allowed_entities".into(),
                                Tag::String("#minecraft:cannot_be_pushed_onto_boats".into()),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                );
            assert_eq!(items.capacity(host, 39, &helmet), 1);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_kept_handler_reaches_nothing_once_its_world_closes() {
        use super::super::minecarts::{CartKind, Minecart};
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            game.carts.push(Minecart::new(
                500,
                CartKind::Chest,
                horizon_core::geometry::Vec3::ZERO,
            ));
            let host: &mut dyn Host = game;
            let entity = host.entity(500).unwrap();
            let items = entity.capability(host, &ENTITY_ITEMS).unwrap();
            game.native_lifecycle.begin_close();
            game.native_lifecycle.world_close_final();
            assert!(entity.capability(game, &ENTITY_ITEMS).is_none());
            let host: &mut dyn Host = game;
            let sticks = ItemResource::of(&ItemStack::new("minecraft:stick", 1));
            let wrote =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert(host, tx, 0, &sticks, 1)));
            assert_eq!(wrote, Ok(0));
        });
    }

    /// The guests' inventories, by connection.
    fn inventories(game: &super::Game<'_>) -> Vec<Vec<Option<ItemStack>>> {
        let guests = &game.lan.as_ref().unwrap().hub.guests;
        guests.iter().map(|g| g.inventory.clone()).collect()
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_players_items_go_where_inventory_add_puts_them_and_an_abort_puts_them_back() {
        with_game(|game| {
            open_to_lan(game);
            let mut player = guest(1);
            player.selected = 4;
            player.inventory[4] = Some(ItemStack::new("minecraft:stick", 60));
            player.inventory[7] = Some(ItemStack::new("minecraft:stick", 10));
            player.inventory[2] = Some(ItemStack::new("minecraft:stone", 1));
            game.lan.as_mut().unwrap().hub.guests.push(player);
            let before = inventories(game);
            let host: &mut dyn Host = game;
            let entity = host.entity(20).unwrap();
            let items = entity.capability(host, &ENTITY_ITEMS).unwrap();
            assert_eq!(items.size(host), 41);
            let sticks = ItemResource::of(&ItemStack::new("minecraft:stick", 1));
            let aborted = host.transaction(|host, tx| {
                assert_eq!(items.insert_any(host, tx, &sticks, 70), 70);
                Err::<(), ()>(())
            });
            assert!(aborted.is_err());
            assert_eq!(inventories(game), before);

            // `Inventory.add`: onto the selected stack (4 more), then the same item in order
            // (54 more), then the first empty slot (the other 12); not onto the stone.
            let host: &mut dyn Host = game;
            let went =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert_any(host, tx, &sticks, 70)));
            assert_eq!(went, Ok(70));
            let after = &inventories(game)[0];
            assert_eq!(after[4], Some(ItemStack::new("minecraft:stick", 64)));
            assert_eq!(after[7], Some(ItemStack::new("minecraft:stick", 64)));
            assert_eq!(after[0], Some(ItemStack::new("minecraft:stick", 12)));
            assert_eq!(after[2], Some(ItemStack::new("minecraft:stone", 1)));
            // Index 39 is the head: a stick doesn't go there.
            let host: &mut dyn Host = game;
            let head =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert(host, tx, 39, &sticks, 1)));
            assert_eq!(head, Ok(0));
        });
    }

    /// One player's offer in a trade: inventory index `.0` must still hold exactly `.1`.
    type Offer = (usize, ItemStack);

    /// Two guests' trade on a menu, as the docs' recipe has it: each offer's index must still
    /// hold exactly what was offered, and goes whole to the other player, or nothing moves.
    fn trade(game: &mut super::Game<'_>, a: &[Offer], b: &[Offer]) -> Result<(), ()> {
        let host: &mut dyn Host = game;
        let items = |host: &mut dyn Host, id| {
            let entity = host.entity(id).unwrap();
            entity.capability(host, &ENTITY_ITEMS).unwrap()
        };
        let (seller, buyer) = (items(host, 20), items(host, 21));
        host.transaction(|host, tx| {
            for (from, to, offer) in [(&seller, &buyer, a), (&buyer, &seller, b)] {
                for (index, stack) in offer {
                    let offered = ItemResource::of(stack);
                    let still = from.resource(host, *index).as_ref() == Some(&offered)
                        && from.amount(host, *index) == stack.count;
                    if !still
                        || from.extract(host, tx, *index, &offered, stack.count) != stack.count
                        || to.insert_any(host, tx, &offered, stack.count) != stack.count
                    {
                        return Err(());
                    }
                }
            }
            Ok(())
        })
    }

    /// A seller with 3 diamonds in slot 0 and a buyer with 5 emeralds in slot 0, beside
    /// `buyer_rest` in every other slot and the off hand.
    fn traders(game: &mut super::Game<'_>, buyer_rest: Option<ItemStack>) {
        open_to_lan(game);
        let mut seller = guest(1);
        seller.inventory[0] = Some(ItemStack::new("minecraft:diamond", 3));
        let mut buyer = guest(2);
        buyer.inventory[0] = Some(ItemStack::new("minecraft:emerald", 5));
        for slot in &mut buyer.inventory[1..36] {
            *slot = buyer_rest.clone();
        }
        buyer.offhand = buyer_rest;
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guests
            .extend([seller, buyer]);
    }

    fn offers() -> ([Offer; 1], [Offer; 1]) {
        (
            [(0, ItemStack::new("minecraft:diamond", 3))],
            [(0, ItemStack::new("minecraft:emerald", 5))],
        )
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_trade_between_two_players_moves_both_offers() {
        with_game(|game| {
            traders(game, None);
            let (diamonds, emeralds) = offers();
            assert_eq!(trade(game, &diamonds, &emeralds), Ok(()));
            // The diamonds go first, into the buyer's first empty slot (1: the emeralds are
            // still in 0), then the emeralds into the seller's emptied slot 0.
            let after = inventories(game);
            assert_eq!(after[0][0], Some(ItemStack::new("minecraft:emerald", 5)));
            assert_eq!(after[1][0], None);
            assert_eq!(after[1][1], Some(ItemStack::new("minecraft:diamond", 3)));
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_trade_into_a_full_inventory_moves_nothing() {
        with_game(|game| {
            // The buyer's other slots and off hand hold stone, so only slot 0 is free once the
            // emeralds leave; the diamonds go before the emeralds and find no room.
            traders(game, Some(ItemStack::new("minecraft:stone", 64)));
            let before = inventories(game);
            let (diamonds, emeralds) = offers();
            assert_eq!(trade(game, &diamonds, &emeralds), Err(()));
            assert_eq!(inventories(game), before);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_stale_offer_moves_nothing() {
        with_game(|game| {
            traders(game, None);
            let (diamonds, emeralds) = offers();
            // The seller used a diamond after offering three, and the buyer moved the emeralds
            // to another slot: each offer no longer matches its index.
            let guests = &mut game.lan.as_mut().unwrap().hub.guests;
            guests[0].inventory[0] = Some(ItemStack::new("minecraft:diamond", 2));
            let before = inventories(game);
            assert_eq!(trade(game, &diamonds, &emeralds), Err(()));
            assert_eq!(inventories(game), before);
            let guests = &mut game.lan.as_mut().unwrap().hub.guests;
            guests[0].inventory[0] = Some(ItemStack::new("minecraft:diamond", 3));
            guests[1].inventory[4] = guests[1].inventory[0].take();
            let before = inventories(game);
            assert_eq!(trade(game, &diamonds, &emeralds), Err(()));
            assert_eq!(inventories(game), before);
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn a_chest_minecarts_items_are_its_slots() {
        use super::super::minecarts::{CartKind, Minecart};
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            let position = horizon_core::geometry::Vec3::new(0.5, 65.0, 0.5);
            game.carts
                .push(Minecart::new(500, CartKind::Chest, position));
            let host: &mut dyn Host = game;
            let cart = host.entity(500).unwrap();
            assert_eq!(cart.kind(host).as_deref(), Some("minecraft:chest_minecart"));
            let items = cart.capability(host, &ENTITY_ITEMS).unwrap();
            assert_eq!(items.size(host), 27);
            let sticks = ItemResource::of(&ItemStack::new("minecraft:stick", 1));
            let aborted = host.transaction(|host, tx| {
                assert_eq!(items.insert_any(host, tx, &sticks, 70), 70);
                Err::<(), ()>(())
            });
            assert!(aborted.is_err());
            assert!(game.carts[0].items.iter().all(Option::is_none));
            let host: &mut dyn Host = game;
            let went =
                host.transaction(|host, tx| Ok::<_, ()>(items.insert_any(host, tx, &sticks, 70)));
            assert_eq!(went, Ok(70));
            assert_eq!(
                game.carts[0].items[0],
                Some(ItemStack::new("minecraft:stick", 64))
            );
            assert_eq!(
                game.carts[0].items[1],
                Some(ItemStack::new("minecraft:stick", 6))
            );
            // A plain minecart has no items.
            game.carts
                .push(Minecart::new(501, CartKind::Rideable, position));
            let host: &mut dyn Host = game;
            let plain = host.entity(501).unwrap();
            assert!(plain.capability(host, &ENTITY_ITEMS).is_none());
        });
    }
}
