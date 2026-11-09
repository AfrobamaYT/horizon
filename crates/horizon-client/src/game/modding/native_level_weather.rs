//! Weather writes for native server mods through a `LevelRef`, on the same path as
//! `/weather`: `ServerLevel.setWeatherParameters` with the weather and thunder hooks, and
//! `Level.setRainLevel` / `setThunderLevel` for how strong the rain and thunder are now. Like
//! the command, they act on the level in place; a parked level, a stale handle or a client
//! returns false. The players see the change on the next host sync.
use super::Game;
use mod_api::LevelRef;

impl Game<'_> {
    /// `ServerLevel.setWeatherParameters(clear, time, raining, thundering)` in the level in
    /// place, as `/weather` and mods set it: `weather_change` may cancel all of it and
    /// `thunder_change` the thunder half.
    pub(in crate::game) fn set_weather_parameters(
        &mut self,
        clear: i32,
        time: i32,
        raining: bool,
        thundering: bool,
    ) {
        let previous = (
            self.world.weather.clear_time,
            self.world.weather.rain_time,
            self.world.weather.thunder_time,
            self.world.weather.raining,
            self.world.weather.thundering,
        );
        let mut cancel = false;
        self.native_weather_change(self.dim.id(), raining, thundering, &mut cancel);
        let old_thundering = self.world.weather.thundering;
        let old_thunder_time = self.world.weather.thunder_time;
        let mut cancel_thunder = false;
        self.native_thunder_change(self.dim.id(), thundering, &mut cancel_thunder);
        if cancel {
            let w = &mut self.world.weather;
            (
                w.clear_time,
                w.rain_time,
                w.thunder_time,
                w.raining,
                w.thundering,
            ) = previous;
        } else {
            // ServerLevel.setWeatherParameters.
            let w = &mut self.world.weather;
            w.clear_time = clear;
            w.rain_time = time;
            w.thunder_time = time;
            w.raining = raining;
            w.thundering = thundering;
            if cancel_thunder {
                w.thunder_time = old_thunder_time;
                w.thundering = old_thundering;
            }
        }
    }
    /// `set_weather_parameters` through a handle of the level in place. False for a negative
    /// time, a parked level, a stale handle or a client.
    pub(super) fn level_set_weather(
        &mut self,
        level: LevelRef,
        clear_ticks: i32,
        weather_ticks: i32,
        raining: bool,
        thundering: bool,
    ) -> bool {
        if !self.level_in_place(level) || clear_ticks < 0 || weather_ticks < 0 {
            return false;
        }
        self.set_weather_parameters(clear_ticks, weather_ticks, raining, thundering);
        true
    }
    /// `Level.setRainLevel(rain)` and `setThunderLevel(thunder)`: both halves of each level
    /// at once, clamped to 0 to 1, so the change shows without fading in. False for a value
    /// that isn't finite, a parked level, a stale handle or a client.
    pub(super) fn level_set_weather_levels(
        &mut self,
        level: LevelRef,
        rain: f32,
        thunder: f32,
    ) -> bool {
        if !self.level_in_place(level) || !rain.is_finite() || !thunder.is_finite() {
            return false;
        }
        let w = &mut self.world.weather;
        let (rain, thunder) = (rain.clamp(0.0, 1.0), thunder.clamp(0.0, 1.0));
        w.rain_level = (rain, rain);
        w.thunder_level = (thunder, thunder);
        true
    }
}

#[cfg(test)]
mod tests {
    use mod_api::{LevelRef, NativeWorldEpoch};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn weather_writes_set_the_level_in_place_and_refuse_others() {
        super::super::native_entity_refs::tests::with_game(|game| {
            let level = game.level(game.dim.id()).unwrap();
            let stale = LevelRef::new(
                NativeWorldEpoch::new(level.epoch().get() + 1),
                level.dimension(),
            );
            let nether = LevelRef::new(level.epoch(), mod_api::DimensionKey::NETHER);

            assert!(level.set_weather(game, 0, 6000, true, true));
            let w = &game.world.weather;
            assert_eq!(
                (
                    w.clear_time,
                    w.rain_time,
                    w.thunder_time,
                    w.raining,
                    w.thundering
                ),
                (0, 6000, 6000, true, true)
            );
            assert!(level.set_weather(game, 1200, 0, false, false));
            let w = &game.world.weather;
            assert_eq!(
                (w.clear_time, w.raining, w.thundering),
                (1200, false, false)
            );
            assert!(!level.set_weather(game, -1, 0, false, false));
            assert!(
                !nether.set_weather(game, 0, 10, true, false),
                "a level not in place"
            );
            assert!(!stale.set_weather(game, 0, 10, true, false));

            assert!(level.set_weather_levels(game, 0.5, 2.0));
            assert_eq!(
                level.weather(game),
                Some((0.5, 1.0)),
                "clamped, without fading"
            );
            assert!(!level.set_weather_levels(game, f32::NAN, 0.0));
            assert!(!stale.set_weather_levels(game, 0.0, 0.0));
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn local_difficulty_and_tick_rate_reach_the_game() {
        super::super::native_entity_refs::tests::with_game(|game| {
            let level = game.level(game.dim.id()).unwrap();
            let stale = LevelRef::new(
                NativeWorldEpoch::new(level.epoch().get() + 1),
                level.dimension(),
            );
            let here = game.local_difficulty([3.0, 64.0, -7.0]);
            let read = level.local_difficulty(game, [3, 64, -7]).unwrap();
            assert_eq!((read.base, read.effective), (here.base, here.effective));
            assert_eq!(read.special_multiplier(), here.special());
            assert_eq!(stale.local_difficulty(game, [3, 64, -7]), None);

            assert!(game.set_tick_rate(40.0));
            assert_eq!(game.tick_stats().0, 40.0);
            assert!(!game.set_tick_rate(0.5));
            assert!(!game.set_tick_rate(f32::NAN));
            assert_eq!(game.clock.rate(), 40.0, "refusals change nothing");
        });
    }
}
