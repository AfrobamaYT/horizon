//! Private custom-book connection state and client network services.
use super::Game;
use horizon_protocol::packets::play::Serverbound as S;
use mod_api::{SendPacketError, ServerboundPlayPacket};
use std::collections::HashMap;
#[derive(Default)]
pub(super) struct NativeNet {
    pub(super) placement: super::native_book_placement::State,
    pub(super) book_connections:
        HashMap<u32, horizon_client::recipe_book::settings_connection::Connection>,
    pub(super) book_view: horizon_client::recipe_book::settings_sync::ClientView,
    pub(super) book_request: u64,
    pub(super) book_ready: bool,
    pub(super) book_retry_at: Option<std::time::Instant>,
}

impl Game<'_> {
    /// A mod's chat line or command (`/` first), sent as `send_chat` sends
    /// it (ClientPacketListener.sendChat / sendCommand), to any server: no
    /// chat history, and no client chat event (vanilla's `ClientChatEvent`
    /// comes only from ChatScreen). An error when the player isn't in a
    /// server's play state or the Chat option blocks the message. API
    /// 404.
    pub(super) fn send_chat(&mut self, message: &str) -> Result<(), String> {
        let Some(remote) = self.remote.as_ref() else {
            return Err("not connected to a server".into());
        };
        // A server that reconfigures the player has them out of the play state.
        if !remote.joined || remote.reconfiguring.is_some() {
            return Err("the player has not joined yet".into());
        }
        let message = message.trim();
        if message.is_empty() {
            return Err("the message is empty".into());
        }
        if !self.chat_abilities().allows(message) {
            return Err("the Chat option blocks this message".into());
        }
        self.send_signed_chat_input(message);
        Ok(())
    }

    /// Opens the chat screen with `text` typed in and the cursor at its end,
    /// as T and / open it (`ChatScreen` with its initial text): a command
    /// macro can fill in `/tp ` for the player to finish. Nothing is sent; the
    /// player edits and sends it, or presses Escape. The box keeps its limit
    /// (256 characters) and its character filter. An error with no player in a
    /// world, or while another screen is open. API @API:open_chat@.
    pub(super) fn open_chat(&mut self, text: &str) -> Result<(), String> {
        if self.player.is_none() {
            return Err("no player in a world".into());
        }
        if self.screen != super::Screen::Playing {
            return Err("another screen is open".into());
        }
        self.begin_chat_screen(text, false);
        Ok(())
    }

    /// Sends `packet` to the server as it is, to any server, a vanilla one
    /// included: a plugin channel's payload (`ServerboundCustomPayloadPacket`)
    /// or any other play packet. Nothing counts it, and the game doesn't apply
    /// it on its side: a packet that needs a sequence number, a signature or
    /// the client's own state is the mod's to get right. An error when the
    /// player isn't in a server's play state, and for an unknown custom payload
    /// past the serverbound discard limit. Registered engine channels retain
    /// their own codec constraints. API 409.
    pub(super) fn send_packet(
        &mut self,
        packet: ServerboundPlayPacket,
    ) -> Result<(), SendPacketError> {
        let remote = self.remote.as_ref().ok_or(SendPacketError::NotConnected)?;
        if !remote.joined || remote.reconfiguring.is_some() {
            return Err(SendPacketError::NotInPlay);
        }
        let prepared = remote.session.prepare_mod_packet(packet)?;
        self.after_commit(super::transactions::After::Packet(prepared));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        Audio, Game, Options, Progress, Settings, execution_role::ExecutionRole, load_resources,
    };
    use super::{SendPacketError, ServerboundPlayPacket};
    use horizon_protocol::quiet;
    use horizon_protocol::{buf::Writer, connection::Pipe};
    use mod_api::Identifier;

    /// A mod's packet goes out only from a joined player, and a custom
    /// payload only within vanilla's size limit.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn mod_packets_need_a_joined_player_and_a_payload_within_the_limit() {
        let root = std::path::PathBuf::from(
            std::env::var_os("HORIZON_TEST_RESOURCES")
                .expect("set HORIZON_TEST_RESOURCES to prepared resources"),
        );
        let settings = Settings::default();
        let loaded = load_resources(
            horizon_assets::resource_source::ResourceRoot::directory(root.clone()),
            Vec::new(),
            None,
            &root,
            &settings,
            std::time::Instant::now(),
            false,
            ExecutionRole::Client,
            &Progress::new(),
            &mut |_, _| unreachable!("no menu handoff"),
        )
        .unwrap();
        let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
        options.resources = root.clone();
        options.execution_role = ExecutionRole::Client;
        let mut game = Game::new(
            &loaded.resources,
            loaded.pack,
            loaded.gui.unwrap(),
            options,
            settings,
            Audio::silent(&horizon_assets::resource_source::ResourceSource::directory(
                root.to_path_buf(),
            )),
            None,
        )
        .unwrap();
        let payload = |len| {
            let mut body = Writer::new();
            body.string("voicechat:request_secret");
            body.bytes(&vec![0; len]);
            ServerboundPlayPacket::new(
                Identifier::parse("minecraft:custom_payload").unwrap(),
                body.data,
            )
        };

        assert!(game.send_packet(payload(4)).is_err());
        let (client, _server) = Pipe::pair();
        game.connect_local(client, "PacketTest").unwrap();
        assert!(game.send_packet(payload(4)).is_err());
        game.remote.as_mut().unwrap().joined = true;
        assert!(game.send_packet(payload(4)).is_ok());
        assert!(game.send_packet(payload(quiet::MAX_PAYLOAD)).is_ok());
        assert!(game.send_packet(payload(quiet::MAX_PAYLOAD + 1)).is_err());
        let mut pong = Writer::new();
        pong.i32(7);
        assert_eq!(
            game.send_packet(ServerboundPlayPacket::new(
                Identifier::parse("minecraft:pong").unwrap(),
                pong.data.clone(),
            )),
            Ok(())
        );
        pong.bytes(&[0]);
        assert_eq!(
            game.send_packet(ServerboundPlayPacket::new(
                Identifier::parse("minecraft:pong").unwrap(),
                pong.data,
            )),
            Err(SendPacketError::InvalidBody)
        );
        let unknown = Identifier::parse("test:unknown_packet").unwrap();
        assert_eq!(
            game.send_packet(ServerboundPlayPacket::new(unknown.clone(), Vec::new())),
            Err(SendPacketError::UnknownPacket { id: unknown })
        );
    }
}
