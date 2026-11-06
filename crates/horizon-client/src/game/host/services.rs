//! The game's side of the `services` area (`horizon-mod-api/src/host/services.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ServicesServerHost for Game<'_> {
    fn commands(&mut self) -> &mut dyn Commands {
        mod_api::__private::check_access("commands", mod_api::__private::LoadAccess::WorldOnly);
        self
    }
    fn levels(&self) -> &dyn Levels {
        mod_api::__private::check_access("levels", mod_api::__private::LoadAccess::WorldOnly);
        self
    }
    fn messages(&self) -> &dyn Messages {
        mod_api::__private::check_access("messages", mod_api::__private::LoadAccess::WorldOnly);
        self
    }
    fn players(&self) -> &dyn Players {
        mod_api::__private::check_access("players", mod_api::__private::LoadAccess::WorldOnly);
        self
    }
}
impl ServicesClientHost for Game<'_> {
    fn client_session(&self) -> &dyn ClientSession {
        mod_api::__private::check_access(
            "client_session",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self
    }
    fn render_assets(&self) -> &dyn RenderAssets {
        mod_api::__private::check_access(
            "render_assets",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self
    }
}
impl ClientSession for Game<'_> {
    fn client_server(&self) -> Option<ClientServer> {
        mod_api::__private::check_access(
            "client_server",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_client_server()
    }
    fn client_language(&self) -> Option<String> {
        mod_api::__private::check_access(
            "client_language",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_client_language()
    }
    fn client_frame_stats(&self) -> Option<ClientFrameStats> {
        mod_api::__private::check_access(
            "client_frame_stats",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_client_frame_stats()
    }
}
impl Commands for Game<'_> {
    fn feedback(&mut self, line: Text) {
        mod_api::__private::check_access("feedback", mod_api::__private::LoadAccess::WorldOnly);
        self.native_feedback(line)
    }
    fn run_command(&mut self, line: &str) -> bool {
        mod_api::__private::check_access("run_command", mod_api::__private::LoadAccess::WorldOnly);
        self.native_run_command(line)
    }
}
impl Levels for Game<'_> {
    fn list_levels(&self) -> Vec<LevelRef> {
        mod_api::__private::check_access("list_levels", mod_api::__private::LoadAccess::WorldOnly);
        self.native_list_levels()
    }
}
impl Messages for Game<'_> {
    fn text_allowed(&self, text: &str) -> Option<bool> {
        mod_api::__private::check_access("text_allowed", mod_api::__private::LoadAccess::WorldOnly);
        self.native_text_allowed(text)
    }
    fn broadcast(&self, line: Text) {
        mod_api::__private::check_access("broadcast", mod_api::__private::LoadAccess::WorldOnly);
        self.native_broadcast(line)
    }
}
impl Players for Game<'_> {
    fn server_players(&self) -> Vec<PlayerRef> {
        mod_api::__private::check_access(
            "server_players",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_server_players()
    }
    fn player_by_name(&self, name: &str) -> Option<PlayerRef> {
        mod_api::__private::check_access(
            "player_by_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_by_name(name)
    }
    fn acting_player(&self) -> Option<PlayerRef> {
        mod_api::__private::check_access(
            "acting_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_acting_player()
    }
}
impl RenderAssets for Game<'_> {
    fn model(&self, id: &Identifier) -> Result<ModelRef, ModelError> {
        mod_api::__private::check_access(
            "model",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_model(id)
    }
}
