//! A player's game mode and permission level for native server mods,
//! through a `PlayerRef` (docs/native-mods.md): the `Player` reads the
//! handle's game mode reads don't give, and the game mode write. Reads
//! return None and the write false for a stale handle or a player who isn't
//! online here; the write also refuses a dead player and, inside another
//! player's turn, anyone but that player.
use super::{Game, game_mode};
use horizon_client::mod_api::PlayerRef;

impl Game<'_> {
    /// The player's permission level, 0 to 4 (`Player.permissions()`,
    /// `PlayerList.getProfilePermissions`): an op's level, the
    /// single-player owner's 4 with commands allowed. API
    /// @API:player_modes@.
    pub(super) fn native_player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        if self.native_player_ref(player.uuid()) != Some(player) {
            return None;
        }
        let guest = self
            .lan
            .iter()
            .flat_map(|h| h.hub.guests.iter())
            .find(|g| g.placed && g.uuid == player.uuid());
        Some(match guest {
            Some(g) => g.permission,
            // `IntegratedServer.getProfilePermissions` for the owner.
            None if self.allow_commands() => 4,
            None => 0,
        })
    }
    /// `Player.hasInfiniteMaterials`: the `instabuild` ability, which
    /// creative mode gives. API @API:player_modes@.
    pub(super) fn native_player_has_infinite_materials(&self, player: PlayerRef) -> Option<bool> {
        self.native_player_is_creative(player)
    }
    /// `Player.canUseGameMasterBlocks`: creative and a permission level of
    /// at least 2, as command blocks, structure blocks and jigsaws ask.
    /// API @API:player_modes@.
    pub(super) fn native_player_can_use_game_master_blocks(
        &self,
        player: PlayerRef,
    ) -> Option<bool> {
        let creative = self.native_player_is_creative(player)?;
        Some(creative && self.native_player_permission_level(player)? >= 2)
    }
    /// `ServerPlayer.setGameMode`: switches the player to `mode` (0
    /// survival, 1 creative, 2 adventure, 3 spectator) as `/gamemode` does,
    /// and `player_game_mode_changed` hears it first and may keep the old
    /// one. False when nothing changed: the same mode, a mode out of range,
    /// a mod that canceled, or a player it can't reach. API
    /// @API:player_modes@.
    pub(super) fn native_player_set_game_mode(&mut self, player: PlayerRef, mode: i32) -> bool {
        if !(0..=3).contains(&mode) || self.native_player_ref(player.uuid()) != Some(player) {
            return false;
        }
        let mode = game_mode::mode_from_id(mode);
        let mut changed = false;
        self.native_as_player(player.uuid(), |game| changed = game.set_game_mode(mode));
        changed
    }
}
