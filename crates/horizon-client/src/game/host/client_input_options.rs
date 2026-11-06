//! The game's side of the `client_input_options` area (`horizon-mod-api/src/host/client_input_options.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientInputOptionsClientHost for Game<'_> {
    fn input_options(&self) -> ClientInputOptions {
        mod_api::__private::check_access(
            "input_options",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_input_options()
    }
}
