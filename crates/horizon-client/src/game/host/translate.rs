//! The game's side of the `translate` area (`horizon-mod-api/src/host/translate.rs`).
use crate::game::Game;
use mod_api::prelude::*;
impl TranslateClientHost for Game<'_> {
    fn translate(&self, key: &str) -> Option<String> {
        mod_api::__private::check_access("translate", mod_api::__private::LoadAccess::ClientSetup);
        self.native_translate(key)
    }
}
