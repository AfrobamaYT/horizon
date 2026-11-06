//! The game's side of the `reload_resources` area (`horizon-mod-api/src/host/reload_resources.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ReloadResourcesServerHost for Game<'_> {
    fn reload_resources(&self) -> &dyn ServerReloadResources {
        mod_api::__private::check_access(
            "reload_resources",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_reload_resources()
    }
}
