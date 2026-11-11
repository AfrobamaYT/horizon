//! `player_start_spectating_entity` and `player_stop_spectating_entity` heard through a guest's
//! camera (`ServerPlayer.setCamera`): one start per new target, one stop when the view returns.
use mod_api::{
    Hooks, ModEntry, PlayerStartSpectatingEntityEvent, PlayerStopSpectatingEntityEvent, ServerHost,
};
use std::sync::{Arc, Mutex};

/// One heard moment: start or stop, the spectator's UUID and name, the target's id.
type Heard = (&'static str, u128, String, i32);

struct Probe(Arc<Mutex<Vec<Heard>>>);

#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn player_start_spectating_entity(
        &self,
        _: &mut dyn ServerHost,
        event: &mut PlayerStartSpectatingEntityEvent<'_>,
    ) {
        self.0.lock().unwrap().push((
            "start",
            event.player,
            event.name.to_owned(),
            event.entity_id,
        ));
    }
    fn player_stop_spectating_entity(
        &self,
        _: &mut dyn ServerHost,
        event: &mut PlayerStopSpectatingEntityEvent<'_>,
    ) {
        self.0
            .lock()
            .unwrap()
            .push(("stop", event.player, event.name.to_owned(), event.entity_id));
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_guests_camera_is_heard_starting_once_and_stopping() {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let mods = vec![ModEntry::new("spectating", Box::new(Probe(heard.clone())))];
    mod_api::testing::with_mods(mods.into(), || {
        super::native_guest_turn_tests::with_guests(|game| {
            let take = || std::mem::take(&mut *heard.lock().unwrap());
            // Guest 12 (index 0) watches guest 13's entity.
            game.guest_set_camera(0, Some(13));
            assert_eq!(take(), [("start", 12, "guest12".to_owned(), 13)]);
            // The same target again changes nothing and is not heard.
            game.guest_set_camera(0, Some(13));
            assert!(take().is_empty());
            // Their own entity is no camera: it is the return to the player.
            game.guest_set_camera(0, Some(12));
            assert_eq!(take(), [("stop", 12, "guest12".to_owned(), 13)]);
            // Without a camera, returning to the player hears nothing.
            game.guest_set_camera(0, None);
            assert!(take().is_empty());
        });
    });
}
