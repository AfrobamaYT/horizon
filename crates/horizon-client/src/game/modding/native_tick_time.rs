//! The tick's spare time, as mods read it (`MinecraftServer.haveTime`).
use super::Game;
use std::time::Instant;

impl Game<'_> {
    /// `MinecraftServer.haveTime`: the current server tick has not reached the
    /// moment the next one is due. A client, and a server outside a scheduled
    /// tick, always has time, as NeoForge's client `LevelTickEvent.hasTime` does.
    pub(super) fn server_has_time(&self) -> bool {
        !self.is_logical_server()
            || self
                .server_tick_deadline
                .is_none_or(|end| Instant::now() < end)
    }
}

#[cfg(test)]
mod tests {
    use crate::game::native_entity_refs::tests::{with_client_game, with_game};
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_server_has_time_until_its_tick_deadline_passes() {
        with_game(|game| {
            game.server_tick_deadline = None;
            assert!(game.server_has_time());
            game.server_tick_deadline = Some(Instant::now() + Duration::from_secs(60));
            assert!(game.server_has_time());
            game.server_tick_deadline = Some(Instant::now() - Duration::from_millis(1));
            assert!(!game.server_has_time());
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_client_level_always_has_time() {
        with_client_game(|game| {
            game.server_tick_deadline = Some(Instant::now() - Duration::from_millis(1));
            assert!(game.server_has_time());
        });
    }
}
