use super::*;
use mod_api::{
    ClientHost, InteractionAttempt, InteractionKey, InteractionKeyMappingTriggeredEvent,
};

struct KeyControl {
    cancel: bool,
    swing: bool,
    hand: Hand,
    calls: Arc<Mutex<Vec<Hand>>>,
    packets: Arc<Mutex<Vec<String>>>,
}
#[mod_api::__subscriptions]
impl Hooks for KeyControl {
    fn client_packet_send(
        &self,
        _: &dyn ClientHost,
        e: &mut mod_api::ClientPacketSendEvent<'_>,
    ) -> Flow {
        self.packets.lock().unwrap().push(e.packet.id.to_string());
        Flow::Continue
    }
    fn interaction_key_mapping_triggered(
        &self,
        _: &mut dyn ClientHost,
        e: &mut InteractionKeyMappingTriggeredEvent,
    ) -> Flow {
        self.calls.lock().unwrap().push(e.hand);
        if e.hand == self.hand {
            e.swing_hand = self.swing;
            if self.cancel {
                return Flow::Cancel;
            }
        }
        Flow::Continue
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn interaction_key_literal_matrix_preserves_actions_and_swing_independently() {
    for (cancel, swing, actions, swings) in [
        (false, false, 1, 0),
        (false, true, 1, 1),
        (true, false, 0, 0),
        (true, true, 0, 1),
    ] {
        for (key, hand, attempt) in [
            (InteractionKey::Use, Hand::Main, InteractionAttempt::Press),
            (InteractionKey::Use, Hand::Off, InteractionAttempt::Press),
            (
                InteractionKey::Attack,
                Hand::Main,
                InteractionAttempt::Press,
            ),
            (InteractionKey::Attack, Hand::Main, InteractionAttempt::Held),
        ] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let mods = vec![mod_api::ModEntry::new(
                "key_matrix",
                Box::new(KeyControl {
                    cancel,
                    swing,
                    hand,
                    calls: calls.clone(),
                    packets: Arc::default(),
                }),
            )]
            .into();
            mod_api::testing::with_mods(mods, || {
                with_interaction_game(true, |game| {
                    let stone = game
                        .pack
                        .assets
                        .registry
                        .default_state("minecraft:stone")
                        .unwrap();
                    let lever = game
                        .pack
                        .assets
                        .registry
                        .default_state("minecraft:lever")
                        .unwrap();
                    game.world.set(0, 63, 0, stone).unwrap();
                    game.world.set(0, 64, 0, lever).unwrap();
                    game.forced_hit = Some(hit());
                    let before = game.remote.as_ref().unwrap().sequence;
                    let before_swings = game.swings_started;
                    if key == InteractionKey::Attack && attempt == InteractionAttempt::Press {
                        game.remote_start_attack();
                    } else if key == InteractionKey::Attack {
                        game.remote_continue_attack(true);
                    } else if hand == Hand::Main {
                        game.remote_use(attempt);
                        if cancel {
                            assert_eq!(*calls.lock().unwrap(), [Hand::Main]);
                        }
                    } else {
                        // Enter the off-hand turn after the main-hand item has passed.
                        game.inventory[game.selected] = None;
                        game.offhand = Some(ItemStack::new("minecraft:stone", 1));
                        game.world.set(0, 64, 0, stone).unwrap();
                        game.remote_use_hands(attempt);
                    }
                    assert_eq!(
                        game.remote.as_ref().unwrap().sequence - before,
                        actions,
                        "{key:?} {hand:?} {attempt:?} cancel={cancel} swing={swing}"
                    );
                    assert_eq!(
                        game.swings_started - before_swings,
                        swings,
                        "{key:?} {hand:?} {attempt:?} cancel={cancel} swing={swing}"
                    );
                })
            });
        }
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn pick_keeps_packet_veto_and_never_swings() {
    for cancel in [false, true] {
        let packets = Arc::new(Mutex::new(Vec::new()));
        let mods = vec![mod_api::ModEntry::new(
            "pick",
            Box::new(KeyControl {
                cancel,
                swing: true,
                hand: Hand::Main,
                calls: Arc::default(),
                packets: packets.clone(),
            }),
        )]
        .into();
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
                use horizon_client::interaction_input::InteractionWorld;
                super::super::RemoteInputWorld(game).pick();
                assert_eq!(
                    packets
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|p| p.ends_with("pick_item_from_block"))
                        .count(),
                    usize::from(!cancel)
                );
                assert_eq!(game.swings_started, 0);
            })
        });
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn no_listener_preserves_natural_swing_and_unsuccessful_use_stays_quiet() {
    mod_api::testing::with_mods(Vec::new().into(), || {
        with_interaction_game(true, |game| {
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            let lever = game
                .pack
                .assets
                .registry
                .default_state("minecraft:lever")
                .unwrap();
            game.world.set(0, 63, 0, stone).unwrap();
            game.world.set(0, 64, 0, lever).unwrap();
            game.forced_hit = Some(hit());
            let sequence = game.remote.as_ref().unwrap().sequence;
            game.remote_use(InteractionAttempt::Press);
            assert_eq!(game.remote.as_ref().unwrap().sequence - sequence, 1);
            assert_eq!(game.swings_started, 1);
            assert_eq!(
                game.remote.as_ref().unwrap().interaction_swing,
                [true, true]
            );
        })
    });
    let mods = vec![mod_api::ModEntry::new(
        "natural",
        Box::new(KeyControl {
            cancel: false,
            swing: true,
            hand: Hand::Main,
            calls: Arc::default(),
            packets: Arc::default(),
        }),
    )]
    .into();
    mod_api::testing::with_mods(mods, || {
        with_interaction_game(true, |game| {
            game.inventory[game.selected] = None;
            game.offhand = None;
            game.forced_hit = None;
            game.remote_use(InteractionAttempt::Press);
            assert_eq!(game.swings_started, 0);
        })
    });
}
