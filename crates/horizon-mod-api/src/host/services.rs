//! The `services` area: host methods the game implements (`game/host/services.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `services` area, implemented by the game and reached through [`ServerHost`].
pub trait ServicesServerHost {
    /// The commands service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn commands(&mut self) -> &mut dyn Commands {
        crate::phase::unprovided(
            "ServerHost::commands",
            "commands",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// The levels service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn levels(&self) -> &dyn Levels {
        crate::phase::unprovided(
            "ServerHost::levels",
            "levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// The messages service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn messages(&self) -> &dyn Messages {
        crate::phase::unprovided(
            "ServerHost::messages",
            "messages",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// The players service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn players(&self) -> &dyn Players {
        crate::phase::unprovided(
            "ServerHost::players",
            "players",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `services` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait ServicesClientHost {
    /// The client_session service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn client_session(&self) -> &dyn ClientSession {
        crate::phase::unprovided(
            "ClientHost::client_session",
            "client_session",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// The render_assets service.
    /// Not on wasm: a frame resource; its methods are the imports.
    fn render_assets(&self) -> &dyn RenderAssets {
        crate::phase::unprovided(
            "ClientHost::render_assets",
            "render_assets",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
}
/// The client_session service, reached through its host.
pub trait ClientSession {
    /// Side: client.
    /// The connected server, not a stale reconnect target. None outside a
    /// logged-in client world. External display names are not retained; memory
    /// connections have no address. Friend-invite secrets are redacted.
    fn client_server(&self) -> Option<ClientServer> {
        crate::phase::unprovided(
            "ClientSession::client_server",
            "client_server",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The applied language code, not a pending options selection. None outside
    /// a logged-in client world. Read is_paused for the applied pause state.
    fn client_language(&self) -> Option<String> {
        crate::phase::unprovided(
            "ClientSession::client_language",
            "client_language",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Existing local frame counters; None outside a logged-in client world.
    /// This read collects no history and sends nothing anywhere.
    fn client_frame_stats(&self) -> Option<ClientFrameStats> {
        crate::phase::unprovided(
            "ClientSession::client_frame_stats",
            "client_frame_stats",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
/// The commands service, reached through its host.
pub trait Commands {
    /// Side: server.
    /// A line of the command's answer, inside `native_perform_command`:
    /// sent to the player whose command it is after the call, as vanilla
    /// commands' feedback is.
    fn feedback(&mut self, line: Text) {
        crate::phase::unprovided(
            "Commands::feedback",
            "feedback",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Runs `line` (no slash) as the server's console does: permission
    /// level 4, from the overworld's spawn, with no output. A teleport to
    /// another level (`execute in minecraft:the_nether run tp Alex 0 70 0`)
    /// happens at the next tick's start, as the console's. Whether it ran
    /// without failing. During a guest's turn, live players (including the
    /// acting player) are reachable; displaced outer players are excluded
    /// from name, UUID and selector resolution on this path only.
    fn run_command(&mut self, line: &str) -> bool {
        crate::phase::unprovided(
            "Commands::run_command",
            "run_command",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The levels service, reached through its host.
pub trait Levels {
    /// Side: server.
    /// Loaded server levels, in vanilla dimension order. Empty without a server world.
    fn list_levels(&self) -> Vec<LevelRef> {
        crate::phase::unprovided(
            "Levels::list_levels",
            "list_levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The messages service, reached through its host.
pub trait Messages {
    /// Side: server.
    /// Whether the server's chat filter (`server.toml` `[chat_filter]`)
    /// lets `text` through: true without a filter, None inside a player's
    /// hook (where the server is out of reach).
    fn text_allowed(&self, text: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Messages::text_allowed",
            "text_allowed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A system line in every player's chat and the server's log.
    fn broadcast(&self, line: Text) {
        crate::phase::unprovided(
            "Messages::broadcast",
            "broadcast",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The players service, reached through its host.
pub trait Players {
    /// Side: server.
    /// The players on this server, in the order they joined. Empty on a
    /// client and in a world nobody else can join.
    fn server_players(&self) -> Vec<PlayerRef> {
        crate::phase::unprovided(
            "Players::server_players",
            "server_players",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Find a connected player by their account name, case-insensitively.
    fn player_by_name(&self, name: &str) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "Players::player_by_name",
            "player_by_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The player whose action or command a hook hears, as their state is
    /// swapped in (`native_perform_command`, `native_right_click_block`,
    /// `native_use_on`, `native_break_block`...): None outside such a
    /// hook.
    fn acting_player(&self) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "Players::acting_player",
            "acting_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The render_assets service, reached through its host.
pub trait RenderAssets {
    /// Side: client.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// Resolve a registered standalone model once in `client_setup` and again in
    /// `client_resources_reloaded`, not each frame. Resource-pack overrides and other
    /// namespaces use the normal shared model store; no resource is read or baked here.
    fn model(&self, id: &Identifier) -> Result<ModelRef, ModelError> {
        crate::phase::unprovided(
            "RenderAssets::model",
            "model",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
}
