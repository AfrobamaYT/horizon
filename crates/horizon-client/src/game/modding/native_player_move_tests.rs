//! `player_move`: the hook hears a held report with the player's state swapped in, and its
//! veto decides what the hub does with it.
use super::native_guest_turn_tests::with_guests;
use horizon_client::lan::hub::{Accepted, PendingMove};
use mod_api::{Flow, Hooks, PlayerMoveEvent, ServerHost};
use std::sync::{Arc, Mutex};

type Heard = Arc<Mutex<Vec<(u128, [f64; 3], [f32; 2], [f64; 3], [f32; 2])>>>;

/// Records each report and vetoes one that goes past x = 10, as a world wall does.
struct Wall(Heard);

#[mod_api::__subscriptions]
impl Hooks for Wall {
    fn player_move(&self, _: &mut dyn ServerHost, event: &mut PlayerMoveEvent) -> Flow {
        self.0.lock().unwrap().push((
            event.player,
            event.from,
            event.from_rotation,
            event.to,
            event.to_rotation,
        ));
        if event.to[0] > 10.0 {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

fn hold(game: &mut super::Game<'_>, conn: u32, to: [f64; 3], yaw: f32) {
    let hub = &mut game.lan.as_mut().unwrap().hub;
    let i = hub.guests.iter().position(|g| g.conn == conn).unwrap();
    let (teleports, dimension) = (hub.guests[i].teleport_counter, hub.guests[i].dimension.clone());
    let report = Accepted {
        to,
        yaw,
        pitch: 0.0,
        flags: 1,
        stands: true,
        floating: false,
    };
    let held = PendingMove {
        report,
        since: hub.ticks,
        teleports,
        dimension,
    };
    hub.guests.update(i, |g| g.pending_move = Some(held));
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_veto_corrects_the_player_and_a_pass_commits_the_report() {
    let heard = Heard::default();
    let mods = vec![mod_api::ModEntry::new("wall", Box::new(Wall(heard.clone())))].into();
    mod_platform::testing::with_mods(mods, || {
        with_guests(|game| {
            // A report inside the wall: heard with the exact values, then committed.
            hold(game, 13, [5.0, 64.0, 5.0], 40.0);
            game.lan_player_move(13);
            assert_eq!(
                std::mem::take(&mut *heard.lock().unwrap()),
                [(13, [3.0, 64.0, 5.0], [0.0, 0.0], [5.0, 64.0, 5.0], [40.0, 0.0])]
            );
            let g = game.lan.as_ref().unwrap().hub.guest(13).unwrap();
            assert_eq!((g.position, g.yaw), ([5.0, 64.0, 5.0], 40.0));
            assert!(g.pending_move.is_none());
            // A report past it: vetoed, the player stays and the client is corrected.
            hold(game, 13, [20.0, 64.0, 5.0], 90.0);
            game.lan_player_move(13);
            assert_eq!(heard.lock().unwrap().len(), 1);
            let g = game.lan.as_ref().unwrap().hub.guest(13).unwrap();
            assert_eq!((g.position, g.yaw), ([5.0, 64.0, 5.0], 40.0));
            assert!(g.awaiting_teleport.is_some(), "corrected through the teleport");
            // Nothing pending any more: the mods are not asked again.
            heard.lock().unwrap().clear();
            game.lan_player_move(13);
            assert!(heard.lock().unwrap().is_empty());
        });
    });
}
