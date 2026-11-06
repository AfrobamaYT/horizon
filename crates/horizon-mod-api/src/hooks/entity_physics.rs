//! The `entity_physics` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Whether the entity's moves pass through blocks (`Entity.noPhysics`): a player's
    /// while it spectates, false for a mob, an arrow's or a trident's own flag (set once a
    /// loyal trident turns back), and true for the ender dragon, area-effect clouds, ominous
    /// item spawners and shulker bullets. Returns `None` for other kinds, whose flag Horizon
    /// does not keep between ticks (items and armor stands among them), and for unavailable
    /// or stale handles.
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_physics::Game<'_>::native_entity_no_physics"
    )]
    fn entity_no_physics();
}
