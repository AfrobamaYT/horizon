//! The `network` area: host methods the game implements (`game/host/network.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `network` area, implemented by the game and reached through [`ServerHost`].
pub trait NetworkServerHost {
    /// Side: server.
    /// Sends `data` on `channel` (`mod_id:name`, at most 30 000 bytes, at
    /// most 32 a player a tick) to `player`'s client, at the tick's end, from
    /// any server hook. Ok means accepted: a player who left or isn't on a
    /// Horizon client doesn't get it.
    fn net_send_to_player(
        &mut self,
        player: u128,
        channel: &str,
        data: &[u8],
    ) -> Result<(), String> {
        crate::phase::unprovided(
            "ServerHost::net_send_to_player",
            "net_send_to_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `network` area, implemented by the game and reached through [`ClientHost`].
pub trait NetworkClientHost {
    /// Side: client.
    /// The client half: sends `data` on `channel` to the server (the same
    /// caps, 32 a tick). An error when the server doesn't carry mod messages
    /// (a vanilla server), and until the player has joined: a server says it
    /// carries them during configuration, before it can take one.
    fn net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::net_send_to_server",
            "net_send_to_server",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// A mod's chat line or command (`/` first), sent as `send_chat` sends
    /// it (ClientPacketListener.sendChat / sendCommand), to any server: no
    /// chat history, and no client chat event (vanilla's `ClientChatEvent`
    /// comes only from ChatScreen). An error when the player isn't in a
    /// server's play state or the Chat option blocks the message. API
    /// 404.
    fn send_chat(&mut self, message: &str) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::send_chat",
            "send_chat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Sends `packet` to the server as it is, to any server, a vanilla one
    /// included: a plugin channel's payload (`ServerboundCustomPayloadPacket`)
    /// or any other play packet. Nothing counts it, and the game doesn't apply
    /// it on its side: a packet that needs a sequence number, a signature or
    /// the client's own state is the mod's to get right. An error when the
    /// player isn't in a server's play state, and for a custom payload past
    /// `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE`, which a server
    /// disconnects for. API 409.
    fn send_packet(&mut self, packet: ServerboundPlayPacket) -> Result<(), SendPacketError> {
        crate::phase::unprovided(
            "ClientHost::send_packet",
            "send_packet",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
