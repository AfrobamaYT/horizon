//! Events, explosions and lightning in a level for native server mods,
//! through a `LevelRef` (docs/native-mods.md, "Entity and level handles"):
//! `Level.levelEvent`, `Level.gameEvent`, `Level.explode` and a summoned
//! lightning bolt. Like the level's block writes, they act on the level in
//! place, the one the hook runs in; a parked level, a stale handle or a
//! client returns false.
use super::{Game, native_explosions::NativeBlast};
use horizon_client::mod_api::{LevelRef, NativeExplosionInteraction};
use horizon_client::sculk::Context;

/// A position inside the world's horizontal and vertical bounds.
fn in_bounds(at: [f64; 3]) -> bool {
    at.iter().all(|v| v.is_finite())
        && at[0].abs() < 30_000_000.0
        && at[2].abs() < 30_000_000.0
        && at[1].abs() < 20_000_000.0
}

impl Game<'_> {
    /// Whether `level` is this open's level in place, on the server.
    fn native_level_in_place(&self, level: LevelRef) -> bool {
        self.remote.is_none()
            && (self.serving.is_some() || self.options.dedicated.is_some())
            && self.native_lifecycle.epoch() == Some(level.epoch())
            && self.dim.id() == level.dimension()
    }
    /// Plays level event `id` at `pos` with `data` for the players near it
    /// (`Level.levelEvent(null, id, pos, data)`): `1000`s are sounds, `2000`s
    /// particles, as vanilla's `LevelEvent` numbers them (`2001` breaks a
    /// block with state id `data`). API @API:level_events@.
    pub(super) fn native_level_level_event(
        &mut self,
        level: LevelRef,
        id: i32,
        pos: [i32; 3],
        data: i32,
    ) -> bool {
        if !self.native_level_in_place(level) {
            return false;
        }
        self.level_event_at(id, pos, data);
        true
    }
    /// Sends game event `event` (`minecraft:block_change`...) from `at`, as
    /// `Level.gameEvent(null, event, at)` does: sculk sensors, shriekers,
    /// wardens and allays in range hear it. False for an event vanilla
    /// doesn't have. API @API:level_events@.
    pub(super) fn native_level_game_event(
        &mut self,
        level: LevelRef,
        event: &str,
        at: [f64; 3],
    ) -> bool {
        let Some(event) = horizon_client::sculk::intern(event) else {
            return false;
        };
        if !self.native_level_in_place(level) || !in_bounds(at) {
            return false;
        }
        self.post_game_event(event, at, Context::default());
        true
    }
    /// Explodes at `center` with no source entity, as `Level.explode(null,
    /// x, y, z, radius, fire, interaction)` does (`native_explode`): `fire`
    /// lights some of the air around, `interaction` says what happens to
    /// blocks. False for a chunk that isn't loaded or a radius outside
    /// `0 < radius <= 32`. API @API:level_events@.
    pub(super) fn native_level_explode(
        &mut self,
        level: LevelRef,
        center: [f64; 3],
        radius: f32,
        fire: bool,
        interaction: NativeExplosionInteraction,
    ) -> bool {
        if !self.native_level_in_place(level) {
            return false;
        }
        let blast = match interaction {
            NativeExplosionInteraction::None => NativeBlast::Keep,
            NativeExplosionInteraction::Block => NativeBlast::Block,
            NativeExplosionInteraction::Tnt => NativeBlast::Tnt,
        };
        self.native_explode(level.dimension(), center, radius, fire, blast)
    }
    /// Strikes lightning at `at`, as summoning a `minecraft:lightning_bolt`
    /// does: it sets fire, hurts and converts what it hits (a pig to a
    /// zombified piglin...) and every player hears the thunder. With
    /// `visual_only` it only flashes and sounds (`setVisualOnly`). False for a
    /// chunk that isn't loaded. API @API:level_events@.
    pub(super) fn native_level_strike_lightning(
        &mut self,
        level: LevelRef,
        at: [f64; 3],
        visual_only: bool,
    ) -> bool {
        if !self.native_level_in_place(level) || !in_bounds(at) {
            return false;
        }
        let chunk = ((at[0].floor() as i32) >> 4, (at[2].floor() as i32) >> 4);
        if !self.world.chunks.contains_key(&chunk) {
            return false;
        }
        self.spawn_bolt(at, visual_only);
        true
    }
}
