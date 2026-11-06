//! The game's side of the `send_packet` area (`horizon-mod-api/src/host/send_packet.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl SendPacketClientHost for Game<'_> {
    fn send_packet(
        &mut self,
        packet: ServerboundPlayPacket,
    ) -> Result<(), SendPacketError> {
        mod_api::__private::check_access(
            "send_packet",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_send_packet(packet)
    }
}
