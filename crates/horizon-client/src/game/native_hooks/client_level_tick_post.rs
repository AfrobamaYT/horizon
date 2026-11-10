use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// Once a client level tick, after the level's own work ran and it
    /// returned normally (`ClientTickEvents.END_LEVEL_TICK`).
    pub(in crate::game) fn native_client_level_tick_post(&mut self, level: LevelRef) {
        self.fire_client_level_tick_post(level);
    }
}
