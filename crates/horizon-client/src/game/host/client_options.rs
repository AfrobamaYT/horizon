//! The game's side of the `client_options` area (`horizon-mod-api/src/host/client_options.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientOptionsClientHost for Game<'_> {
    fn input_options(&self) -> ClientInputOptions {
        mod_api::__private::check_access(
            "input_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_input_options()
    }
    fn graphics_options(&self) -> ClientGraphicsOptions {
        mod_api::__private::check_access(
            "graphics_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_graphics_options()
    }
    fn gui_options(&self) -> ClientGuiOptions {
        mod_api::__private::check_access(
            "gui_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_gui_options()
    }
    fn chat_options(&self) -> ClientChatOptions {
        mod_api::__private::check_access(
            "chat_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_chat_options()
    }
    fn audio_options(&self) -> ClientAudioOptions {
        mod_api::__private::check_access(
            "audio_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_audio_options()
    }
}
