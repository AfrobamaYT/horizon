//! The game's side of the `host_send_map` area (`horizon-mod-api/src/host/host_send_map.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl HostSendMapBackendMut for Game<'_> {
    fn player_send_map(&mut self, player: PlayerRef, map_id: i32, colors: &[u8]) -> bool {
        mod_api::__private::check_access(
            "player_send_map",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_send_map(player, map_id, colors)
    }
}
