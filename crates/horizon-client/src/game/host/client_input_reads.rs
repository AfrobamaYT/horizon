//! The game's side of the `client_input_reads` area (`horizon-mod-api/src/host/client_input_reads.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ClientInputReadsClientHost for Game<'_> {
    fn playing(&self) -> bool {
        mod_api::__private::check_access("playing", mod_api::__private::LoadAccess::ClientSetup);
        self.native_playing()
    }
    fn mapping(&self, name: &str) -> Option<NativeKeyMapping> {
        mod_api::__private::check_access("mapping", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mapping(name)
    }
    fn modifiers(&self) -> ClientModifiers {
        mod_api::__private::check_access("modifiers", mod_api::__private::LoadAccess::ClientSetup);
        self.native_modifiers()
    }
}
