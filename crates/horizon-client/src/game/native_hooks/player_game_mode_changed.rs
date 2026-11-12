use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// A server player's game mode is about to change (NeoForge's
    /// `PlayerChangeGameModeEvent`): from `from` to `to`, both the shared
    /// `GameMode` used by commands and `PlayerRef`. `player` is the UUID and
    /// `name` the name. Heard for
    /// the `/gamemode` command, the F3+F4 switcher's packet and a hardcore
    /// world's respawn into spectator, once for each change to a different
    /// mode; not for the world's default mode a player joins with. Set `to`
    /// to send the player to another mode instead: listeners run in order and
    /// each sees the mode the one before left. Return `Flow::Cancel`, or
    /// leave `to` at the player's current mode, and the player keeps their
    /// mode (nothing is sent, the command counts it as no change); the
    /// command's message names the mode it asked for, as vanilla's does. For a change the game makes inside
    /// a player's turn their state is swapped in; for the hardcore respawn
    /// it is not.
    pub(in crate::game) fn native_player_game_mode_changed(
        &mut self,
        player: u128,
        name: &str,
        from: GameMode,
        to: &mut GameMode,
        cancel: &mut bool,
    ) {
        self.fire_player_game_mode_changed(player, name, from, to, cancel);
    }
}

#[cfg(test)]
mod player_game_mode_changed_tests {
    use mod_api::{Flow, GameMode, Hooks, ModEntry, PlayerGameModeChangedEvent, ServerHost};

    /// Sends a switch to creative to adventure, and one to spectator back
    /// to the mode the player has.
    struct Lobby;
    #[mod_api::__subscriptions]
    impl Hooks for Lobby {
        fn player_game_mode_changed(
            &self,
            _: &mut dyn ServerHost,
            event: &mut PlayerGameModeChangedEvent<'_>,
        ) -> Flow {
            match *event.to {
                GameMode::Creative => *event.to = GameMode::Adventure,
                GameMode::Spectator => *event.to = event.from,
                _ => {}
            }
            Flow::Continue
        }
    }

    /// A redirect is what the player gets; a redirect to the current mode
    /// changes nothing.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_listener_redirects_the_new_game_mode() {
        let mods: mod_api::Mods = vec![ModEntry::new("lobby", Box::new(Lobby))].into();
        mod_platform::testing::with_mods(mods, || {
            crate::game::native_guest_turn_tests::with_guests(|game| {
                let mode = |game: &crate::game::Game<'_>| {
                    game.lan.as_ref().unwrap().hub.guest(13).unwrap().game_mode
                };
                game.as_guest(13, |g| {
                    g.set_game_mode(GameMode::Survival);
                    assert!(g.set_game_mode(GameMode::Creative));
                    assert_eq!(g.mode, GameMode::Adventure);
                })
                .unwrap();
                assert_eq!(mode(game), GameMode::Adventure.id());
                let changed = game
                    .as_guest(13, |g| g.set_game_mode(GameMode::Spectator))
                    .unwrap();
                assert!(!changed, "a redirect to the current mode is no change");
                assert_eq!(mode(game), GameMode::Adventure.id());
            });
        });
    }
}
