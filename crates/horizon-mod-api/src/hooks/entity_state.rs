//! The `entity_state` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Whether the player is using secondary use (sneak), not whether its pose is crouching.
    /// On the client, the local player's live semantic sneak state is read; obtain its entity
    /// from `host.local_player()` or `host.entity(id)`. On the server this reads the
    /// entity's shift-key flag, including a persistent fake's action setup.
    #[helper(
        side = "both",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_shift_key_down"
    )]
    fn entity_is_shift_key_down();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_crouching"
    )]
    fn entity_is_crouching();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_sprinting"
    )]
    fn entity_is_sprinting();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_in_water"
    )]
    fn entity_is_in_water();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_in_lava"
    )]
    fn entity_is_in_lava();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_fire_immune"
    )]
    fn entity_fire_immune();
    /// Remaining fire ticks for players, mobs, armor stands and item entities.
    /// Returns `None` for other kinds and unavailable or stale handles; zero is a
    /// valid value. Negative values retain the entity's fire immunity countdown.
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_fire_ticks"
    )]
    fn entity_fire_ticks();
    /// Remaining portal cooldown in ticks (`Entity.getPortalCooldown`) for players and
    /// the kinds that go through portals: mobs, items, experience orbs, boats, minecarts,
    /// primed TNT, falling blocks, arrows and the thrown items and fireballs a chunk saves,
    /// ender pearls (End gateways only), fireworks and tridents. Zero is a valid value.
    /// Returns `None` for other kinds (among them armor stands, wind charges and the ender
    /// dragon), on a client, and for unavailable or stale handles.
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_portal_cooldown"
    )]
    fn entity_portal_cooldown();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_remaining_fire_ticks"
    )]
    fn entity_set_remaining_fire_ticks();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_clear_fire"
    )]
    fn entity_clear_fire();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_air_supply"
    )]
    fn entity_air_supply();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_air_supply"
    )]
    fn entity_set_air_supply();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_max_air_supply"
    )]
    fn entity_max_air_supply();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_silent"
    )]
    fn entity_is_silent();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_silent"
    )]
    fn entity_set_silent();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_glowing"
    )]
    fn entity_glowing();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_no_gravity"
    )]
    fn entity_no_gravity();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_ticks_frozen"
    )]
    fn entity_ticks_frozen();
    /// Ticks the entity has lived (`Entity.tickCount`) for mobs, item entities,
    /// experience orbs and minecarts. Returns `None` for other kinds and
    /// unavailable or stale handles. This is not an ageable mob's growth age or an
    /// item's despawn age.
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_tick_count"
    )]
    fn entity_tick_count();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_glowing"
    )]
    fn entity_set_glowing();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_no_gravity"
    )]
    fn entity_set_no_gravity();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_ticks_frozen"
    )]
    fn entity_set_ticks_frozen();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_set_invisible"
    )]
    fn entity_set_invisible();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_is_invisible"
    )]
    fn entity_is_invisible();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_state::Game<'_>::native_entity_pose"
    )]
    fn entity_pose();
}
