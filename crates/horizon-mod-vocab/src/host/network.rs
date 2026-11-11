//! Host method declarations; access defaults and game forwarders are generated.
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `network` area, implemented by the game and reached through [`crate::ClientHost`].
#[horizon_mod_api_macros::host_area]
pub trait NetworkClientHost {
    /// Side: client.
    /// Opens the chat screen with `text` typed in for the player to finish and
    /// send, as T and / open it; nothing is sent. Returns an error with no
    /// player in a world or while another screen is open.
    #[access(WorldOnly)]
    fn open_chat(&mut self, text: &str) -> Result<(), String>;
    /// Side: client.
    /// Adds `line` to this client's chat HUD and history (vanilla's
    /// `ChatComponent.addClientSystemMessage`): it shows here only, so
    /// nothing is sent to the server. The Chat option's server-message filter
    /// doesn't hide it (`GuiMessageSource.SYSTEM_CLIENT`), and
    /// `client_chat_received` is not told: vanilla fires that event from
    /// `ChatListener` for lines the server sent, not for lines the client adds
    /// itself. Returns an error if the text exceeds the boundary's size or
    /// nesting limits; rejected text is not added.
    #[access(WorldOnly)]
    fn chat_line(&mut self, line: Text) -> Result<(), TextFormError>;
    /// Side: client.
    /// A mod's chat line or command (`/` first), sent as `send_chat` sends
    /// it (ClientPacketListener.sendChat / sendCommand), to any server: no
    /// chat history, and no client chat event (vanilla's `ClientChatEvent`
    /// comes only from ChatScreen). An error when the player isn't in a
    /// server's play state or the Chat option blocks the message. API
    /// 404.
    #[access(WorldOnly)]
    fn send_chat(&mut self, message: &str) -> Result<(), String>;
    /// Side: client.
    /// Sends `packet` to the server as it is, to any server, a vanilla one
    /// included: a plugin channel's payload (`ServerboundCustomPayloadPacket`)
    /// or any other play packet. Nothing counts it, and the game doesn't apply
    /// it on its side: a packet that needs a sequence number, a signature or
    /// the client's own state is the mod's to get right. An error when the
    /// player isn't in a server's play state, and for an unknown custom payload
    /// past `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE` (the discard
    /// fallback's limit). Registered engine channels use their own codecs'
    /// constraints, not that byte cap. API 409.
    #[access(WorldOnly)]
    fn send_packet(&mut self, packet: ServerboundPlayPacket) -> Result<(), SendPacketError>;
}
