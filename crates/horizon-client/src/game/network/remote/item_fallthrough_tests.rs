use super::*;

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn targeted_item_fallthrough_keeps_hand_and_fail_vetoes_item_action() {
    let observer = Observer::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "fallthrough_observer",
        Box::new(observer.clone()),
    )]
    .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        for client in [false, true] {
            with_interaction_game(client, |game| {
                let side = if client {
                    LogicalSide::Client
                } else {
                    LogicalSide::Server
                };
                game.forced_hit = Some(hit());
                for hand in [Hand::Main, Hand::Off] {
                    *observer.0.lock().unwrap() = Controls {
                        air_answer: Some(Answer::Fail),
                        ..Controls::default()
                    };
                    let apple = ItemStack::new("minecraft:apple", 1);
                    assert_eq!(game.right_click_item(hand, &apple), Some(Answer::Fail));
                    assert_eq!(observer.0.lock().unwrap().steps, [Step::Air(side, hand)]);
                    assert!(game.using.is_none());
                }
            });
        }
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn block_pass_falls_through_but_fail_and_success_stop_the_apple() {
    let observer = Observer::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "block_fallthrough",
        Box::new(observer.clone()),
    )]
    .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        with_interaction_game(true, |game| {
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            game.world.set(0, 64, 0, stone).unwrap();
            game.forced_hit = Some(hit());
            for (answer, expected) in [(Answer::Pass, 1), (Answer::Fail, 0), (Answer::Success, 0)] {
                game.inventory[game.selected] = Some(ItemStack::new("minecraft:apple", 1));
                *observer.0.lock().unwrap() = Controls {
                    block_answer: Some(Answer::Pass),
                    item_on_answer: Some(answer),
                    air_answer: Some(Answer::Fail),
                    ..Controls::default()
                };
                if !game.remote_use_block(Hand::Main) {
                    game.remote_use_item(Hand::Main);
                }
                let state = observer.0.lock().unwrap();
                assert_eq!(
                    state
                        .steps
                        .iter()
                        .filter(|step| matches!(step, Step::Air(LogicalSide::Client, Hand::Main)))
                        .count(),
                    expected
                );
                assert!(!state.steps.contains(&Step::ItemPart));
                assert!(game.using.is_none());
            }
        });
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn entity_pass_falls_through_but_success_stops_the_apple() {
    let observer = Observer::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "entity_fallthrough",
        Box::new(observer.clone()),
    )]
    .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        for (entity, expected) in [("minecraft:zombie", 1), ("minecraft:villager", 0)] {
            with_interaction_game(true, |game| {
                game.inventory[game.selected] = Some(ItemStack::new("minecraft:apple", 1));
                game.offhand = None;
                let kind = game.res.mob_types().get(entity).unwrap().clone();
                game.mobs.push(horizon_client::mobs::Mob::new(
                    42,
                    kind,
                    Vec3::new(0.5, 64.0, 5.5),
                    0.0,
                ));
                assert!(game.remote_use_targets_entity());
                // Zombie interaction passes; the awake adult villager succeeds.
                // Entity CONSUME prediction belongs to #2931 and is deliberately excluded.
                *observer.0.lock().unwrap() = Controls {
                    air_answer: Some(Answer::Fail),
                    ..Controls::default()
                };
                game.remote_use(mod_api::InteractionAttempt::Press);
                let state = observer.0.lock().unwrap();
                let hands: Vec<_> = state
                    .steps
                    .iter()
                    .filter_map(|s| match s {
                        Step::Air(LogicalSide::Client, hand) => Some(*hand),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    hands,
                    if expected == 1 {
                        vec![Hand::Main]
                    } else {
                        vec![]
                    },
                    "{entity}"
                );
            });
        }
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn air_use_fires_once_and_passing_hands_fire_in_main_off_order() {
    let observer = Observer::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "air_fallthrough",
        Box::new(observer.clone()),
    )]
    .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        with_interaction_game(true, |game| {
            game.inventory[game.selected] = Some(ItemStack::new("minecraft:apple", 1));
            assert!(!game.remote_use_targets_entity());
            assert!(game.pick().is_none());
            for (offhand, expected) in [
                (false, vec![Hand::Main]),
                (true, vec![Hand::Main, Hand::Off]),
            ] {
                game.offhand = offhand.then(|| ItemStack::new("minecraft:apple", 1));
                *observer.0.lock().unwrap() = Controls {
                    air_answer: Some(Answer::Pass),
                    ..Controls::default()
                };
                game.remote_use(mod_api::InteractionAttempt::Press);
                let state = observer.0.lock().unwrap();
                let hands: Vec<_> = state
                    .steps
                    .iter()
                    .filter_map(|s| match s {
                        Step::Air(LogicalSide::Client, hand) => Some(*hand),
                        _ => None,
                    })
                    .collect();
                assert_eq!(hands, expected);
            }
        });
    });
}

fn with_packet_game(test: impl FnOnce(&mut Game<'_>)) {
    with_game(|game| {
        game.options.dedicated = Some(Vec::new());
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        game.lan_host(
            LanSettings {
                port: 0,
                guest_commands: false,
                force_game_mode: false,
            },
            NetConfig {
                listen: false,
                announce: false,
                lod_distance: 0,
                ..NetConfig::lan(0, String::new(), 2)
            },
            HubConfig {
                default_game_mode: 1,
                guest_commands: false,
                force_game_mode: false,
                hardcore: false,
                view_distance: 2,
                simulation_distance: 2,
                dimension: game.dim.id().to_owned(),
                allow_flight: true,
                seed: 0,
                max_players: 2,
                integrated: true,
                levels: game.level_shapes,
            },
        )
        .unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut guest = Guest::new(
            Joined {
                conn: 12,
                uuid: 12,
                name: "interaction_guest".into(),
                properties: Vec::new(),
                info: ClientInformation::new(8),
                brand: String::new(),
                sender: PlayerSender::new(tx, Arc::new(AtomicUsize::new(0))),
                latency: Arc::new(AtomicI32::new(0)),
                address: "127.0.0.1:1".parse().unwrap(),
                expected: false,
            },
            12,
            [0.5, 64.0, 3.5],
            10,
        );
        guest.placed = true;
        guest.game_mode = 1;
        game.lan.as_mut().unwrap().hub.guests.push(guest);
        test(game);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn server_use_item_packet_fires_once_after_use_item_on_with_block_crosshair() {
    use horizon_protocol::packets::play::{self, Serverbound as S};
    let observer = Observer::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "packet_fallthrough",
        Box::new(observer.clone()),
    )]
    .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        with_packet_game(|game| {
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            game.world.set(0, 64, 0, stone).unwrap();
            game.forced_hit = Some(hit());
            game.lan.as_mut().unwrap().hub.guests.update(0, |g| {
                g.inventory[g.selected] = Some(ItemStack::new("minecraft:apple", 1));
            });
            *observer.0.lock().unwrap() = Controls {
                item_on_answer: Some(Answer::Pass),
                air_answer: Some(Answer::Fail),
                ..Controls::default()
            };
            game.lan_guest_packet(
                12,
                S::UseItemOn(play::UseItemOn {
                    hand: 0,
                    pos: [0, 64, 0],
                    face: 1,
                    cursor: [0.5, 1.0, 0.5],
                    inside: false,
                    world_border: false,
                    sequence: 1,
                }),
            );
            assert!(
                observer
                    .0
                    .lock()
                    .unwrap()
                    .steps
                    .iter()
                    .any(|s| matches!(s, Step::Phase(_, LogicalSide::Server)))
            );
            assert!(
                !observer
                    .0
                    .lock()
                    .unwrap()
                    .steps
                    .iter()
                    .any(|s| matches!(s, Step::Air(_, _)))
            );
            game.as_guest(12, |g| assert!(g.pick().is_some())).unwrap();
            game.lan_guest_packet(
                12,
                S::UseItem(play::UseItem {
                    hand: 0,
                    sequence: 2,
                    yaw: 0.0,
                    pitch: 0.0,
                }),
            );
            let state = observer.0.lock().unwrap();
            let hands: Vec<_> = state
                .steps
                .iter()
                .filter_map(|s| match s {
                    Step::Air(LogicalSide::Server, hand) => Some(*hand),
                    _ => None,
                })
                .collect();
            assert_eq!(hands, [Hand::Main]);
        });
    });
}
