use super::*;
use std::cell::RefCell;

thread_local! {
    /// The players whose `player_bed_leave` the mods are hearing on this
    /// thread: a listener that wakes the same player again wakes them
    /// without a second event.
    static WAKING: RefCell<Vec<u128>> = const { RefCell::new(Vec::new()) };
}

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// [`Self::native_player_bed_leave`] for the player acting now, who sleeps
    /// in `self.sleeping`; its name is copied only when a mod listens.
    pub(in crate::game) fn native_acting_bed_leave(
        &mut self,
        wake_immediately: bool,
        update_level: bool,
    ) {
        if let (Some(head), Some(player)) = (self.sleeping, self.lan_acting_uuid)
            && mod_api::__private::listened(mod_api::HookId::PlayerBedLeave)
        {
            let name = self.lan_acting_name.clone().unwrap_or_default();
            self.native_player_bed_leave(player, &name, head, wake_immediately, update_level);
        }
    }

    /// `PlayerWakeUpEvent`: a server player is getting out of the bed at
    /// `pos` (`Player.stopSleepInBed`), heard first, while they still read
    /// as sleeping there. `wake_immediately` is vanilla's flag for skipping
    /// the wake-up fade. Morning uses (false, false), explicit leave/sneak
    /// and bed-rule refusal use (false, true), and generic stop-sleeping,
    /// death and living-entity continuation failure use (true, true).
    pub(in crate::game) fn native_player_bed_leave(
        &mut self,
        player: u128,
        name: &str,
        pos: [i32; 3],
        wake_immediately: bool,
        update_level: bool,
    ) {
        // A listener that wakes this player again (`stop_sleeping`) must not
        // be told again; the caller re-reads the sleep state afterwards.
        if WAKING.with_borrow(|waking| waking.contains(&player)) {
            return;
        }
        struct Heard(u128);
        impl Drop for Heard {
            fn drop(&mut self) {
                WAKING.with_borrow_mut(|waking| waking.retain(|p| *p != self.0));
            }
        }
        WAKING.with_borrow_mut(|waking| waking.push(player));
        let _heard = Heard(player);
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
    /// Records each call; with `again`, also wakes the player itself.
    struct Wake(Heard, bool);
    #[mod_api::__subscriptions]
    impl Hooks for Wake {
        fn player_bed_leave(&self, host: &mut dyn ServerHost, event: &mut PlayerBedLeaveEvent<'_>) {
            let living = host
                .player(event.player)
                .and_then(|player| host.living(player.entity()));
            let pos = living.and_then(|living| living.sleeping_pos(host));
            let flags = (event.wake_immediately, event.update_level, pos);
            self.0.lock().unwrap().push(flags);
            if self.1
                && let Some(living) = living
            {
                living.stop_sleeping(host);
            }
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
        let mods: mod_api::Mods =
            vec![ModEntry::new("wake", Box::new(Wake(heard.clone(), false)))].into();
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

    /// A listener that wakes the player itself (`stop_sleeping`) inside the
    /// event is not told again, and the guest wakes once, by its call.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_listener_waking_the_player_again_is_told_once() {
        let heard = Heard::default();
        let mods: mod_api::Mods =
            vec![ModEntry::new("wake", Box::new(Wake(heard.clone(), true)))].into();
        mod_platform::testing::with_mods(mods, || {
            crate::game::native_guest_turn_tests::with_guests(|game| {
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
                let guest = game.command_guest(12).unwrap();
                assert!(!guest.sleeping && guest.bed.is_none());
                in_bed(game, 12);
                game.wake_guests();
                assert_eq!(
                    std::mem::take(&mut *heard.lock().unwrap()),
                    [(false, false, Some([5, 64, 5]))]
                );
                assert!(!game.command_guest(12).unwrap().sleeping);
            });
        });
    }
}
