//! The `send_packet` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Sends `packet` to the server as it is, to any server, a vanilla one
    /// included: a plugin channel's payload (`ServerboundCustomPayloadPacket`)
    /// or any other play packet. Nothing counts it, and the game doesn't apply
    /// it on its side: a packet that needs a sequence number, a signature or
    /// the client's own state is the mod's to get right. An error when the
    /// player isn't in a server's play state, and for a custom payload past
    /// `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE`, which a server
    /// disconnects for. API 409.
    ///
    /// `ServerboundPlayPacket` owns the `minecraft:<packet_name>` identifier and the
    /// vanilla encoded body, without its packet id, length or compression framing.
    /// `SendPacketError` distinguishes connection/play state, an unknown packet name,
    /// invalid body and oversized custom payload. Every modeled play packet uses the
    /// same game-side decoder and send path; no protocol crate type crosses this API.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_net::Game<'_>::native_send_packet"
    )]
    fn send_packet();
}
