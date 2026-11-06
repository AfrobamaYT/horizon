//! The game's side of the `screen_exclusion_areas` area (`horizon-mod-api/src/host/screen_exclusion_areas.rs`).
use crate::game::Game;
use mod_api::prelude::*;
impl ScreenExclusionAreasClientHost for Game<'_> {
    fn screen_exclusion(&self) -> Vec<[f32; 4]> {
        mod_api::__private::check_access(
            "screen_exclusion",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_exclusion()
    }
}
