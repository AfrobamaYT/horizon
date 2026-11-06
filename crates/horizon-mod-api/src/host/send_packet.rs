//! The `send_packet` area: host methods the game implements (`game/host/send_packet.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `send_packet` area, implemented by the game and reached through [`ClientHost`].
pub trait SendPacketClientHost {
    /// Side: client.
    /// Sends `packet` to the server as it is, to any server, a vanilla one
    /// included: a plugin channel's payload (`ServerboundCustomPayloadPacket`)
    /// or any other play packet. Nothing counts it, and the game doesn't apply
    /// it on its side: a packet that needs a sequence number, a signature or
    /// the client's own state is the mod's to get right. An error when the
    /// player isn't in a server's play state, and for a custom payload past
    /// `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE`, which a server
    /// disconnects for. API 409.
    fn send_packet(
        &mut self,
        packet: ServerboundPlayPacket,
    ) -> Result<(), SendPacketError> {
        crate::phase::unprovided(
            "ClientHost::send_packet",
            "send_packet",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
