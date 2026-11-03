//! A player's permission level for native server mods, through a
//! `PlayerRef` (docs/native-mods.md): the `Player` reads the handle's game
//! mode reads don't give. They return None for a stale handle or a player
//! who isn't online here.
use super::Game;
use horizon_client::mod_api::PlayerRef;

impl Game<'_> {
    /// The player's permission level, 0 to 4 (`Player.permissions()`,
    /// `PlayerList.getProfilePermissions`): an op's level, the
    /// single-player owner's 4 with commands allowed. API
    /// @API:player_modes@.
    pub(super) fn native_player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        Some(match self.player_handle_read(player)? {
            super::native_server::PlayerRead::Guest(g) => g.permission,
            super::native_server::PlayerRead::Acting { .. } if self.lan_acting.is_some() => {
                self.ui.registries.permission
            }
            super::native_server::PlayerRead::Acting { .. } if self.allow_commands() => 4,
            super::native_server::PlayerRead::Acting { .. } => 0,
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
}
