//! The `config` area: host methods the game implements (`game/host/config.rs`).
#![allow(unused_variables)]
use crate::*;
use std::path::PathBuf;
/// The host's methods both sides share of the `config` area, implemented by the game and reached through [`Host`].
pub trait ConfigHost {
    /// Side: both.
    /// Whether the player left mod `mod_id` on in Options > Mods. A mod
    /// turned off does nothing (check this first in each hook) until the
    /// launcher's next build leaves it out.
    fn on(&self, mod_id: &str) -> bool {
        crate::phase::unprovided("Host::on", "on", crate::phase::LoadAccess::WorldOnly)
    }
    /// Side: both.
    /// A mod's setting `key` (one file per key in `native_data_dir`).
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        crate::phase::unprovided(
            "Host::setting",
            "setting",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Keeps a mod's setting `key`.
    fn set_setting(&self, mod_id: &str, key: &str, value: &str) {
        crate::phase::unprovided(
            "Host::set_setting",
            "set_setting",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The calling mod's own files in the game's `config/` folder, beside
    /// `options.txt` (beside `server.properties` on a dedicated server):
    /// its folder `config/<mod id>/` and the files beside it named
    /// `<mod id>.<ext>` or `<mod id>-<rest>`, as Fabric and NeoForge mods
    /// keep theirs. The service is bound to the mod whose hook or part is
    /// running, so a mod never names its id and can't reach another mod's
    /// files; a world's `serverconfig/` is read only through
    /// `config_loading`.
    fn config_files(&self) -> &'static dyn ConfigFiles {
        crate::phase::unprovided(
            "Host::config_files",
            "config_files",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The level in place (`minecraft:overworld`), for a hook that isn't told
    /// which it runs in.
    fn dimension(&self) -> &'static str {
        crate::phase::unprovided(
            "Host::dimension",
            "dimension",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The instance (game) folder, which its players share: `saves/`,
    /// `resourcepacks/`, `shaderpacks/` and `horizon-mods/` are in it, and the
    /// title's screens create worlds and read packs there. A world's own
    /// `datapacks/` is in that world's folder (`saves/<world>/datapacks`).
    /// The folder the game runs in (`--player-dir` moves a player's options
    /// and servers only, not the instance). Nothing is created.
    ///
    /// Not on wasm: a path in the game's file system, which a wasm mod has none of; a wasm mod
    /// reads its own data and assets through reload listeners.
    fn instance_dir(&self) -> PathBuf {
        crate::phase::unprovided(
            "Host::instance_dir",
            "instance_dir",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `config` area, implemented by the game and reached through [`ServerHost`].
pub trait ConfigServerHost {
    /// Side: server.
    /// The server's tick rate (20 unless `/tick rate` changed it) and the
    /// milliseconds its ticks took on average lately, as `/tick query`
    /// reports them. The ticks a second it keeps up is the rate, or
    /// 1000 / milliseconds when that is lower.
    fn tick_stats(&self) -> (f32, f64) {
        crate::phase::unprovided(
            "ServerHost::tick_stats",
            "tick_stats",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The open world's folder (`world/`, beside its `level.dat`): a server
    /// mod keeps its data in a folder of its own there, so the world's
    /// backups carry it. None when no world is open.
    ///
    /// Not on wasm: a path in the game's file system, which a wasm mod has none of; a wasm mod
    /// keeps world data in level attachments.
    fn world_dir(&self) -> Option<PathBuf> {
        crate::phase::unprovided(
            "ServerHost::world_dir",
            "world_dir",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The retained server reload listeners of the last successful reload.
    fn reload_resources(&self) -> &dyn ServerReloadResources {
        crate::phase::unprovided(
            "ServerHost::reload_resources",
            "reload_resources",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `config` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait ConfigClientHost {
    /// Side: client.
    /// The selected language's text for a translation key (`Component.translatable`'s lookup), `None`
    /// when no language layer defines it.
    fn translate(&self, key: &str) -> Option<String> {
        crate::phase::unprovided(
            "ClientHost::translate",
            "translate",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
