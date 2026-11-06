//! The game's side of the `network` area (`horizon-mod-api/src/host/network.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl NetworkServerHost for Game<'_> {
    fn net_send_to_player(
        &mut self,
        player: u128,
        channel: &str,
        data: &[u8],
    ) -> Result<(), String> {
        mod_api::__private::check_access(
            "net_send_to_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_net_send_to_player(player, channel, data)
    }
}
impl NetworkClientHost for Game<'_> {
    fn net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String> {
        mod_api::__private::check_access(
            "net_send_to_server",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_net_send_to_server(channel, data)
    }
    fn send_chat(&mut self, message: &str) -> Result<(), String> {
        mod_api::__private::check_access("send_chat", mod_api::__private::LoadAccess::WorldOnly);
        self.native_send_chat(message)
    }
    fn send_packet(&mut self, packet: ServerboundPlayPacket) -> Result<(), SendPacketError> {
        mod_api::__private::check_access("send_packet", mod_api::__private::LoadAccess::WorldOnly);
        self.native_send_packet(packet)
    }
}
