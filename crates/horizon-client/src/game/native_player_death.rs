//! A server player's last death location for native mods, through a
//! `PlayerRef` (docs/native-mods.md): `Player.getLastDeathLocation` and
//! `setLastDeathLocation`. It is kept in the player's record, saved as
//! `LastDeathLocation`, and sent with their next respawn packet, which is what
//! the recovery compass follows. Reads return None for a stale handle, for a
//! player who isn't online here, and when the player never died. A death in a
//! level that is not loaded reads as None too, and so does any call made while the
//! server has the player list out for a step (the record is not reachable then).
use super::Game;
use horizon_client::mod_api::{LevelRef, PlayerRef};

impl Game<'_> {
    /// The level and block of the player's last death
    /// (`Player.getLastDeathLocation`).
    pub(super) fn native_player_last_death_location(
        &self,
        player: PlayerRef,
    ) -> Option<(LevelRef, [i32; 3])> {
        if self.native_player_ref(player.uuid()) != Some(player) {
            return None;
        }
        let guest = self
            .lan
            .iter()
            .flat_map(|h| h.hub.guests.iter())
            .find(|g| g.placed && g.uuid == player.uuid())?;
        let (dimension, pos) = guest.last_death.as_ref()?;
        Some((self.native_level(dimension)?, *pos))
    }

    /// `Player.setLastDeathLocation`: stores where the player died, or
    /// forgets it for None. The client hears of it with the player's next
    /// respawn, as in vanilla. False in the cases a read gives None.
    pub(super) fn native_player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<(LevelRef, [i32; 3])>,
    ) -> bool {
        if self.native_player_ref(player.uuid()) != Some(player) {
            return false;
        }
        let Some(guest) = self.lan.as_mut().and_then(|h| {
            h.hub
                .guests
                .iter_mut()
                .find(|g| g.placed && g.uuid == player.uuid())
        }) else {
            return false;
        };
        guest.last_death = location.map(|(level, pos)| (level.dimension().to_owned(), pos));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_players_last_death_location_reads_from_their_record() {
        use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
        use horizon_protocol::packets::configuration::ClientInformation;
        use std::sync::{Arc, atomic::AtomicI32, mpsc};

        with_game(|game| {
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

            let host = game.lan.as_mut().unwrap();
            host.hub.guests.push(guest);
            let player = game.native_player_ref(123).unwrap();
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
        });
    }
}
