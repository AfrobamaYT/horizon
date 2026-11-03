//! A server player's last death location for native mods, through a
//! `PlayerRef` (docs/native-mods.md): `Player.getLastDeathLocation` and
//! `setLastDeathLocation`. It is kept in the player's record (swapped into
//! `level.last_death` for the player's turn), saved as `LastDeathLocation`, and
//! sent with their next respawn packet, which is what the recovery compass
//! follows. Players resolve through the shared player-handle path
//! (`native_server.rs`), so reads return None for a stale handle, for a player
//! who isn't online here or whose record is swapped out, and when the player
//! never died. A death in a level that is not loaded reads as None too.
use super::Game;
use horizon_client::mod_api::{LevelRef, PlayerRef};

impl Game<'_> {
    /// The level and block of the player's last death
    /// (`Player.getLastDeathLocation`).
    pub(super) fn native_player_last_death_location(
        &self,
        player: PlayerRef,
    ) -> Option<(LevelRef, [i32; 3])> {
        let (dimension, pos) = self.native_player_last_death_record(player)?;
        Some((self.native_level(&dimension)?, pos))
    }

    /// `Player.setLastDeathLocation`: stores where the player died, or
    /// forgets it for None. The client hears of it with the player's next
    /// respawn, as in vanilla. False in the cases a read gives None, and for
    /// a stale `LevelRef`.
    pub(super) fn native_player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<(LevelRef, [i32; 3])>,
    ) -> bool {
        let stored = match location {
            Some((level, pos)) => {
                if self.native_lifecycle.epoch() != Some(level.epoch()) {
                    return false;
                }
                Some((level.dimension().to_owned(), pos))
            }
            None => None,
        };
        if self.native_player_ref(player.uuid()) != Some(player) {
            return false;
        }
        let mut done = false;
        self.native_as_player(player.uuid(), |game| {
            game.level.last_death = stored;
            done = true;
        }) && done
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;

    use super::super::Game;
    use horizon_client::mod_api::PlayerRef;

    /// Opens the game to a LAN host with one placed guest (UUID 123, conn 1).
    fn add_guest(game: &mut Game<'_>) -> PlayerRef {
        use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
        use horizon_protocol::packets::configuration::ClientInformation;
        use std::sync::{Arc, atomic::AtomicI32, mpsc};

        let settings = super::super::lan::LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        };
        let net = NetConfig {
            announce: false,
            ..NetConfig::lan(0, "DeathTest".into(), 2)
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
        let (tx, _rx) = mpsc::channel();
        let mut guest = Guest::new(
            Joined {
                conn: 1,
                uuid: 123,
                name: "guest".into(),
                properties: Vec::new(),
                info: ClientInformation::new(8),
                brand: String::new(),
                sender: PlayerSender::new(tx, Arc::default()),
                latency: Arc::new(AtomicI32::new(0)),
                address: "127.0.0.1:1".parse().unwrap(),
            },
            20,
            [0.0; 3],
            8,
        );
        guest.placed = true;
        game.lan.as_mut().unwrap().hub.guests.push(guest);
        game.native_player_ref(123).unwrap()
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_players_last_death_location_reads_from_their_record() {
        with_game(|game| {
            let player = add_guest(game);
            // Never died.
            assert_eq!(game.native_player_last_death_location(player), None);

            // A death records the level and block (`ServerPlayer.die`).
            let overworld = game.native_level("minecraft:overworld").unwrap();
            let guest = &mut game.lan.as_mut().unwrap().hub.guests[0];
            guest.last_death = Some(("minecraft:overworld".into(), [4, 70, -9]));
            assert_eq!(
                game.native_player_last_death_location(player),
                Some((overworld, [4, 70, -9]))
            );

            // The setter stores it, and None forgets it.
            assert!(
                game.native_player_set_last_death_location(player, Some((overworld, [1, 2, 3])))
            );
            assert_eq!(
                game.lan.as_ref().unwrap().hub.guests[0].last_death,
                Some(("minecraft:overworld".into(), [1, 2, 3]))
            );
            assert_eq!(
                game.native_player_last_death_location(player),
                Some((overworld, [1, 2, 3]))
            );
            assert!(game.native_player_set_last_death_location(player, None));
            assert_eq!(game.native_player_last_death_location(player), None);

            // A player who is not online here, and a handle of another
            // player's id, read nothing.
            let mut stale = player;
            stale.entity = horizon_client::mod_api::EntityRef::new(
                player.entity.epoch(),
                player.entity.id() + 1,
            );
            game.lan.as_mut().unwrap().hub.guests[0].last_death =
                Some(("minecraft:overworld".into(), [1, 2, 3]));
            assert_eq!(game.native_player_last_death_location(stale), None);
            assert!(!game.native_player_set_last_death_location(stale, None));
            game.lan.as_mut().unwrap().hub.guests[0].placed = false;
            assert_eq!(game.native_player_last_death_location(player), None);
            game.lan.as_mut().unwrap().hub.guests[0].placed = true;

            // A level handle of an earlier open is refused, and nothing is
            // stored.
            game.native_lifecycle
                .begin_open(horizon_client::mod_api::NativeAuthority::IntegratedServer);
            let player = game.native_player_ref(123).unwrap();
            let fresh = game.native_level("minecraft:overworld").unwrap();
            game.lan.as_mut().unwrap().hub.guests[0].last_death = None;
            assert!(
                !game.native_player_set_last_death_location(player, Some((overworld, [7, 8, 9])))
            );
            assert_eq!(game.lan.as_ref().unwrap().hub.guests[0].last_death, None);
            assert!(game.native_player_set_last_death_location(player, Some((fresh, [7, 8, 9]))));
            assert_eq!(
                game.native_player_last_death_location(player),
                Some((fresh, [7, 8, 9]))
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_death_records_the_floored_block_and_dimension() {
        with_game(|game| {
            let player = add_guest(game);
            {
                // Dying in the Nether at a fractional position.
                let guest = &mut game.lan.as_mut().unwrap().hub.guests[0];
                guest.dimension = "minecraft:the_nether".into();
                guest.position = [-1.2, 64.9, 7.1];
                guest.health = 0.0;
            }
            let mut host = game.lan.take().unwrap();
            game.guest_die(&mut host, 0, 1);
            game.lan = Some(host);

            let guest = &game.lan.as_ref().unwrap().hub.guests[0];
            assert!(guest.dead);
            assert_eq!(
                guest.last_death,
                Some(("minecraft:the_nether".into(), [-2, 64, 7]))
            );
            // The nether is not an open level here, so the read finds none.
            assert_eq!(game.native_player_last_death_location(player), None);
        });
    }
}
