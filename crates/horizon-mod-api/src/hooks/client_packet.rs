//! The `client_packet` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A play packet from the server reached the window's game, decoded,
    /// just before the game handles it (`Connection.genericsFtw` handing it
    /// to `ClientPacketListener`): every packet of the play phase, custom
    /// payloads included, in the order the server sent them. Packets the
    /// network thread handles itself are not heard: keep-alives, bundle
    /// delimiters, resource pack pushes and pops, cookies, server links,
    /// transfers, disconnects, the configuration switch, `minecraft:brand`
    /// and far terrain tiles; neither is the configuration phase.
    /// Read-only: the game handles the packet as it would without the mod.
    /// Client-side, on any server, single player's own included.
    /// Every packet exposes its protocol type id. Custom payloads also expose
    /// their channel and raw bytes; no other decoded fields cross the API.
    #[hook(
        side = "client",
        contract = "listen",
        cancellable = false,
        neoforge = "none (NeoForge has no raw play-packet receive event)"
    )]
    fn client_packet();
}
