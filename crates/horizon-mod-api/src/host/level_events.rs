//! The `level_events` area: host methods the game implements (`game/host/level_events.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `level_events` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LevelEventsBackendMut {
    /// Side: server.
    /// Plays level event `id` at `pos` with `data` for the players near it
    /// (`Level.levelEvent(null, id, pos, data)`): `1000`s are sounds, `2000`s
    /// particles, as vanilla's `LevelEvent` numbers them (`2001` breaks a
    /// block with state id `data`). No player is left out, the acting one
    /// included. False for an event that carries a block state (2001, 2014,
    /// 3008) when `data` isn't one.
    fn level_level_event(&mut self, level: LevelRef, id: i32, pos: [i32; 3], data: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_level_event",
            "level_level_event",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sends game event `event` (`minecraft:block_change`...) from `at`, as
    /// `Level.gameEvent(null, event, at)` does: sculk sensors, shriekers,
    /// wardens and allays in range hear it. False for an event vanilla
    /// doesn't have.
    fn level_game_event(&mut self, level: LevelRef, event: &str, at: [f64; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_game_event",
            "level_game_event",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Explodes at `center` with no source entity, as `Level.explode(null,
    /// x, y, z, radius, fire, interaction)` does (`native_explode`): `fire`
    /// lights some of the air around, `interaction` says what happens to
    /// blocks: `None`, `Block` or `Tnt` (vanilla's `Mob` and `Trigger` aren't
    /// offered yet). False for a chunk that isn't loaded or a radius outside
    /// `0 < radius <= 32`.
    fn level_explode(
        &mut self,
        level: LevelRef,
        center: [f64; 3],
        radius: f32,
        fire: bool,
        interaction: NativeExplosionInteraction,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_explode",
            "level_explode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Strikes lightning at `at`, as summoning a `minecraft:lightning_bolt`
    /// does: it sets fire, hurts and converts what it hits (a pig to a
    /// zombified piglin...) and every player hears the thunder. With
    /// `visual_only` it only flashes and sounds (`setVisualOnly`). The
    /// `lightning_strike` hook hears it first with the cause `mod` and may
    /// cancel it. False for a chunk that isn't loaded or a canceled strike.
    fn level_strike_lightning(&mut self, level: LevelRef, at: [f64; 3], visual_only: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_strike_lightning",
            "level_strike_lightning",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
