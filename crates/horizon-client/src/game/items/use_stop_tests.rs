use super::item_use::interact_vibration_tests::with_game;

#[test]
#[ignore = "requires prepared resources"]
fn spectator_stops_use_at_player_feet() {
    let lines = with_game("minecraft:golden_apple", |game| {
        assert!(game.start_using_item());
        game.set_mode(horizon_client::command::args::GameMode::Spectator);
        assert!(game.using.is_none());
    })
    .expect("prepared resources");
    assert_eq!(
        lines,
        [
            "G 10,64,-4 minecraft:item_interact_start",
            "G 10,64,-4 minecraft:item_interact_finish",
        ]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn stopping_spear_use_is_silent() {
    let lines = with_game("minecraft:iron_spear", |game| {
        assert!(game.start_using_item());
        game.stop_active_item_use();
        assert!(game.using.is_none());
    })
    .expect("prepared resources");
    assert!(lines.is_empty(), "{lines:?}");
}

fn guest_action(item: &str, action: u8) -> Vec<String> {
    use super::native_item_places::tests::{guest, open_to_lan};
    use horizon_client::items::ItemStack;
    use horizon_protocol::packets::play::{self, Serverbound};
    with_game(item, |game| {
        game.reset_entity_residency();
        game.native_lifecycle
            .begin_open(super::native_lifecycle::Authority::IntegratedServer);
        open_to_lan(game);
        let mut player = guest(1);
        player.permission = 4;
        player.position = [21.5, 70.0, -8.25];
        player.inventory[0] = Some(ItemStack::new(item, 1));
        game.lan.as_mut().unwrap().hub.guests.push(player);
        game.as_guest(1, |g| assert!(g.start_using_item()));
        match action {
            0 => game.lan_guest_packet(
                1,
                Serverbound::PlayerAction(play::PlayerAction {
                    action: 6,
                    pos: [0; 3],
                    direction: 0,
                    sequence: 0,
                }),
            ),
            1 => game.lan_guest_packet(
                1,
                Serverbound::SetCarriedItem(play::SetCarriedItem { slot: 1 }),
            ),
            2 => {
                let player = game.player(123).unwrap();
                assert!(game.player_set_selected_slot(player, 1));
            }
            3 => {
                game.lan
                    .as_mut()
                    .unwrap()
                    .hub
                    .guests
                    .update(0, |guest| guest.health = 0.0);
                game.guest_die(0, 1);
            }
            4 => {
                game.as_guest(1, |g| assert!(g.run_command("tp @s 12 65 -2")));
            }
            _ => unreachable!(),
        }
        game.as_guest(1, |g| assert_eq!(g.using.is_some(), action == 4));
    })
    .expect("prepared resources")
}

#[test]
#[ignore = "requires prepared resources"]
fn guest_swap_stops_at_guest_feet() {
    assert_eq!(
        guest_action("minecraft:golden_apple", 0),
        [
            "G 21,70,-9 minecraft:item_interact_start",
            "G 21,70,-9 minecraft:item_interact_finish",
        ]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn guest_hotbar_change_stops_at_guest_feet() {
    assert_eq!(
        guest_action("minecraft:golden_apple", 1),
        [
            "G 21,70,-9 minecraft:item_interact_start",
            "G 21,70,-9 minecraft:item_interact_finish",
        ]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn mod_hotbar_change_stops_at_guest_feet() {
    assert_eq!(
        guest_action("minecraft:golden_apple", 2),
        [
            "G 21,70,-9 minecraft:item_interact_start",
            "G 21,70,-9 minecraft:item_interact_finish",
        ]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn spear_hotbar_change_posts_nothing() {
    assert!(guest_action("minecraft:iron_spear", 1).is_empty());
}

#[test]
#[ignore = "requires prepared resources"]
fn guest_death_clear_posts_no_finish() {
    assert_eq!(
        guest_action("minecraft:golden_apple", 3),
        [
            "G 21,70,-9 minecraft:item_interact_start",
            "G 21,70,-9 minecraft:entity_die"
        ]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn axe_disabled_shield_posts_finish() {
    use horizon_client::health::HurtSource;
    use horizon_client::health::{BlockOutcome, NO_ATTACK};
    let lines = with_game("minecraft:shield", |game| {
        assert!(game.start_using_item());
        game.combat.player_attacker = Some(([11.5, 64.0, -3.25], 5.0));
        game.after_block(
            BlockOutcome {
                blocking_item_used: true,
                hurt: false,
                blocked: 4.0,
                shield_damage: 1,
                fully_blocked: true,
                sound: false,
                knockback: false,
                canceled: false,
                dealt: 0.0,
            },
            (None, false),
            HurtSource {
                damage_type: "minecraft:player_attack",
                causing: 1,
                direct: 1,
                position: None,
            },
            &NO_ATTACK,
        );
        assert!(game.using.is_none());
        assert!(game.shield_cooldown > 0);
    })
    .expect("prepared resources");
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.ends_with("minecraft:item_interact_finish"))
            .cloned()
            .collect::<Vec<_>>(),
        ["G 10,64,-4 minecraft:item_interact_finish"]
    );
}

#[test]
#[ignore = "requires prepared resources"]
fn same_dimension_tp_keeps_use() {
    assert_eq!(
        guest_action("minecraft:golden_apple", 4),
        ["G 21,70,-9 minecraft:item_interact_start"]
    );
}
