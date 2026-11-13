//! An API-only mod: hooks and one `[[inject]]` listener. Alone it takes the API route (the
//! prebuilt API executable and the launcher's `horizon_native_mods` library); beside a mod with a
//! `[native]` table row the whole set takes the engine route. It runs the same on both.
use mod_api::{Hooks, ServerHost};

struct Mod;

impl Default for Mod {
    fn default() -> Self {
        eprintln!("fnr: api_only constructed");
        Mod
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_started(&self, _: &mut dyn ServerHost, _: &mut mod_api::ServerStartedEvent) {
        eprintln!("fnr: api_only server_started");
    }
}

pub fn entry_listener(_host: mod_api::HostSide<'_>, _args: &mut mod_points::TickBlockEntitiesArgs) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| eprintln!("fnr: api_only listener at tick_block_entities"));
}
