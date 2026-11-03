//! Real-game regressions for native handles used during a guest's turn.

use super::{lan::LanSettings, native_entity_refs::tests::with_game};
use horizon_client::{
    lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig},
    mobs::Mob,
};
use horizon_core::geometry::Vec3;
use horizon_protocol::packets::configuration::ClientInformation;
use std::sync::{
    Arc,
    atomic::{AtomicI32, AtomicUsize},
};

fn with_guests(test: impl FnOnce(&mut super::Game<'_>)) {
    with_game(|game| {
        let settings = LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        };
        let net = NetConfig {
            listen: false,
            announce: false,
            lod_distance: 0,
            ..NetConfig::lan(0, String::new(), 2)
        };
        let config = HubConfig {
            default_game_mode: 0,
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
        };
        game.lan_host(settings, net, config).unwrap();
        for id in [12, 13] {
            let (tx, _rx) = std::sync::mpsc::channel();
            let mut guest = Guest::new(
                Joined {
                    conn: id,
                    uuid: u128::from(id),
                    name: format!("guest{id}"),
                    properties: Vec::new(),
                    info: ClientInformation::new(8),
                    brand: String::new(),
                    sender: PlayerSender::new(tx, Arc::new(AtomicUsize::new(0))),
                    latency: Arc::new(AtomicI32::new(0)),
                    address: "127.0.0.1:1".parse().unwrap(),
                },
                id as i32,
                [3.0, 64.0, 5.0],
                10,
            );
            guest.placed = true;
            game.lan.as_mut().unwrap().hub.guests.push(guest);
        }
        test(game);
    });
}

/// Before #228, the host was absent during a guest turn and B and the
/// online list were invisible to native handles.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn another_guest_and_online_players_are_visible_during_a_turn() {
    with_guests(|game| {
        let rider = game.native_entity(13).unwrap();
        game.as_guest(12, |game| {
            assert_eq!(rider.position(game), Some([3.0, 64.0, 5.0]));
            let online = game.native_server_players();
            assert_eq!(online.iter().map(|p| p.uuid).collect::<Vec<_>>(), [12, 13]);
            assert_eq!(online[1].position, [3.0, 64.0, 5.0]);
        })
        .unwrap();
    });
}

/// Requires #235's passenger walk as well as #228's host visibility.
#[test]
#[ignore = "needs #235's passenger walk"]
fn another_guest_follows_a_vehicle_teleported_during_a_turn() {
    with_guests(|game| {
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guest_mut(13)
            .unwrap()
            .vehicle = Some(42);
        let horse = game.res.mob_types().get("minecraft:horse").unwrap().clone();
        game.mobs
            .push(Mob::new(42, horse, Vec3::new(3.0, 64.0, 5.0), 0.0));
        // Put B in the horse's seat using the same placement as a normal turn.
        game.as_guest(13, |game| game.position_rider()).unwrap();
        let vehicle = game.native_entity(42).unwrap();
        let rider = game.native_entity(13).unwrap();
        let before = [3.0, 64.0 + (1.44375 - 0.6), 5.0];
        let after = [10.0, 70.0 + (1.44375 - 0.6), -4.0];
        game.as_guest(12, |game| {
            assert_eq!(rider.position(game), Some(before));
            assert!(vehicle.teleport_to(game, [10.0, 70.0, -4.0], None));
            assert_eq!(vehicle.position(game), Some([10.0, 70.0, -4.0]));
            assert_eq!(rider.position(game), Some(after));
            assert_eq!(
                game.lan.as_ref().unwrap().hub.guest(13).unwrap().vehicle,
                Some(42)
            );
        })
        .unwrap();
    });
}
