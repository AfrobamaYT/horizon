use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// `PlayerWakeUpEvent`: a server player is getting out of the bed at
    /// `pos` (`Player.stopSleepInBed`), heard first, while they still read
    /// as sleeping there. `wake_immediately` is vanilla's flag for skipping
    /// the wake-up fade; every Horizon caller passes false, as vanilla's
    /// morning wake and leave-bed button do. `update_level` is false when
    /// the level woke everyone at morning (`ServerLevel.wakeUpAllPlayers`)
    /// and true when the player got up: the Leave Bed button, their bed
    /// gone, a rule that no longer lets them sleep.
    pub(in crate::game) fn native_player_bed_leave(
        &mut self,
        player: u128,
        name: &str,
        pos: [i32; 3],
        wake_immediately: bool,
        update_level: bool,
    ) {
        self.fire_player_bed_leave(player, name, pos, wake_immediately, update_level);
    }
}

#[cfg(test)]
mod player_bed_leave_tests {
    use crate::game::Game;
    use horizon_protocol::packets::play::{PlayerCommand, Serverbound};
    use mod_api::{Hooks, ModEntry, PlayerBedLeaveEvent, ServerHost};
    use std::sync::{Arc, Mutex};

    /// What each call heard: the two flags and where the player still slept.
    type Heard = Arc<Mutex<Vec<(bool, bool, Option<[i32; 3]>)>>>;
    struct Wake(Heard);
    #[mod_api::__subscriptions]
    impl Hooks for Wake {
        fn player_bed_leave(&self, host: &mut dyn ServerHost, event: &mut PlayerBedLeaveEvent<'_>) {
            let pos = match host
                .player(event.player)
                .and_then(|player| host.living(player.entity()))
            {
                Some(living) => living.sleeping_pos(host),
                None => None,
            };
            let flags = (event.wake_immediately, event.update_level, pos);
            self.0.lock().unwrap().push(flags);
        }
    }

    fn in_bed(game: &mut Game<'_>, conn: u32) {
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .update_guest(conn, |g| {
                g.bed = Some([5, 64, 5]);
                g.sleeping = true;
            })
            .unwrap();
    }

    /// The morning wake is `(false, false)` and the Leave Bed packet
    /// `(false, true)`; both are heard while the guest still sleeps there.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn bed_leave_carries_the_wake_flags_before_the_sleep_clears() {
        let heard = Heard::default();
        let mods: mod_api::Mods = vec![ModEntry::new("wake", Box::new(Wake(heard.clone())))].into();
        mod_platform::testing::with_mods(mods, || {
            crate::game::native_guest_turn_tests::with_guests(|game| {
                in_bed(game, 12);
                game.wake_guests();
                assert_eq!(
                    std::mem::take(&mut *heard.lock().unwrap()),
                    [(false, false, Some([5, 64, 5]))]
                );
                in_bed(game, 12);
                let id = game.command_guest(12).unwrap().entity_id;
                let leave = PlayerCommand {
                    id,
                    action: 0,
                    data: 0,
                };
                game.lan_guest_packet(12, Serverbound::PlayerCommand(leave));
                assert_eq!(
                    std::mem::take(&mut *heard.lock().unwrap()),
                    [(false, true, Some([5, 64, 5]))]
                );
                assert!(!game.command_guest(12).unwrap().sleeping);
            });
        });
    }
}
