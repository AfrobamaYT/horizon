//! The game's side of the `config` area (`horizon-mod-api/src/host/config.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use std::path::PathBuf;
impl ConfigHost for Game<'_> {
    fn on(&self, mod_id: &str) -> bool {
        mod_api::__private::check_access("on", mod_api::__private::LoadAccess::WorldOnly);
        self.native_on(mod_id)
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        mod_api::__private::check_access("setting", mod_api::__private::LoadAccess::WorldOnly);
        self.native_setting(mod_id, key)
    }
    fn set_setting(&self, mod_id: &str, key: &str, value: &str) {
        mod_api::__private::check_access("set_setting", mod_api::__private::LoadAccess::WorldOnly);
        self.native_set_setting(mod_id, key, value)
    }
    fn config_files(&self) -> &'static dyn ConfigFiles {
        mod_api::__private::check_access("config_files", mod_api::__private::LoadAccess::WorldOnly);
        Self::native_config_files()
    }
    fn dimension(&self) -> &'static str {
        mod_api::__private::check_access("dimension", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dimension()
    }
    fn instance_dir(&self) -> PathBuf {
        mod_api::__private::check_access("instance_dir", mod_api::__private::LoadAccess::WorldOnly);
        self.native_instance_dir()
    }
}
impl ConfigServerHost for Game<'_> {
    fn tick_stats(&self) -> (f32, f64) {
        mod_api::__private::check_access("tick_stats", mod_api::__private::LoadAccess::WorldOnly);
        self.native_tick_stats()
    }
    fn world_dir(&self) -> Option<PathBuf> {
        mod_api::__private::check_access("world_dir", mod_api::__private::LoadAccess::WorldOnly);
        self.native_world_dir()
    }
    fn reload_resources(&self) -> &dyn ServerReloadResources {
        mod_api::__private::check_access(
            "reload_resources",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_reload_resources()
    }
}
impl ConfigClientHost for Game<'_> {
    fn translate(&self, key: &str) -> Option<String> {
        mod_api::__private::check_access("translate", mod_api::__private::LoadAccess::ClientSetup);
        self.native_translate(key)
    }
}
