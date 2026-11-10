use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// Once a client level tick, before the level's own work runs
    /// (`ClientTickEvents.START_LEVEL_TICK`).
    pub(in crate::game) fn native_client_level_tick_pre(&mut self, level: LevelRef) {
        self.fire_client_level_tick_pre(level);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{
        native_entity_refs::tests::with_client_game, native_lifecycle::NativeLifecycle,
    };
    use horizon_client::level_loading::Reason;
    use horizon_protocol::{connection::Pipe, packets::play::SpawnInfo};
    use mod_api::{
        ClientHost, ClientLevelTickPostEvent, ClientLevelTickPreEvent, Hooks, LevelRef, ModEntry,
    };
    use std::sync::{Arc, Mutex};

    struct Moments(Arc<Mutex<Vec<(&'static str, LevelRef)>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Moments {
        fn client_level_tick_pre(
            &self,
            _: &mut dyn ClientHost,
            event: &mut ClientLevelTickPreEvent,
        ) {
            self.0.lock().unwrap().push(("pre", event.level));
        }
        fn client_level_tick_post(
            &self,
            _: &mut dyn ClientHost,
            event: &mut ClientLevelTickPostEvent,
        ) {
            self.0.lock().unwrap().push(("post", event.level));
        }
    }

    /// One pre then one post for each client level tick, naming the level in
    /// place, and nothing before a tick or for a client with no level.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn client_level_tick_fires_pre_then_post_around_each_tick() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mods = vec![ModEntry::new("moments", Box::new(Moments(heard.clone())))].into();
        mod_api::testing::with_mods(mods, || {
            with_client_game(|game| {
                game.native_lifecycle = NativeLifecycle::default();
                let (client, _server) = Pipe::pair();
                game.connect_local(client, "LevelTickTest").unwrap();
                let spawn = SpawnInfo {
                    dimension_type: 0,
                    dimension: "minecraft:overworld".into(),
                    seed: 0,
                    game_type: 1,
                    previous_game_type: None,
                    debug: false,
                    flat: true,
                    last_death: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                };
                game.enter_remote_dimension(&spawn, false, Reason::Other);
                game.remote.as_mut().unwrap().logged_in = true;
                let level = game.level("minecraft:overworld").unwrap();

                // Login alone is no tick, and neither is a level with no tick.
                assert!(heard.lock().unwrap().is_empty());

                let before = game.world.game_time;
                game.tick_remote_level().unwrap();
                let after = game.world.game_time;
                assert_eq!(after, before + 1, "the tick advanced the world's time");
                assert_eq!(
                    *heard.lock().unwrap(),
                    [("pre", level), ("post", level)],
                    "the level's own work ran between them"
                );

                heard.lock().unwrap().clear();
                game.tick_remote_level().unwrap();
                assert_eq!(*heard.lock().unwrap(), [("pre", level), ("post", level)]);
            });
        });
    }
}
