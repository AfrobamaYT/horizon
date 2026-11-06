//! The `host_send_map` area: host methods the game implements (`game/host/host_send_map.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `host_send_map` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait HostSendMapBackendMut {
    /// Side: server.
    /// Sends the player `uuid` the whole picture of map `map_id`: 128 x 128
    /// colours (`MapColor` ids times 4 plus the shade, row by row), scale 0,
    /// locked, no decorations, as `ClientboundMapItemDataPacket`. A filled
    /// map the mod gives with that `minecraft:map_id` then shows it in hand
    /// and in item frames; the server keeps nothing (a map id with no saved
    /// map is never sent again by the game), so send it again after the
    /// player joins. Use ids far from the world's own (they count up from 0).
    /// False for a player not here, or `colors` not 16384 long.
    fn player_send_map(&mut self, player: PlayerRef, map_id: i32, colors: &[u8]) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_send_map",
            "player_send_map",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
