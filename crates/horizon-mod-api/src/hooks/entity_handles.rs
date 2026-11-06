//! The `entity_handles` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity"
    )]
    fn entity();
    #[helper(
        side = "both",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_level"
    )]
    fn level();
    /// The name of a dimension's key (`minecraft:the_nether`). None for a key the game never gave out.
    #[helper(
        side = "both",
        receiver = "dimension_key",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_dimension_key_id"
    )]
    fn dimension_key_id();
    #[helper(
        side = "both",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_position"
    )]
    fn entity_position();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_block_position"
    )]
    fn entity_block_position();
    #[helper(
        side = "both",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_rotation"
    )]
    fn entity_rotation();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_kind"
    )]
    fn entity_kind();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_uuid"
    )]
    fn entity_uuid();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_refs::Game<'_>::native_entity_level"
    )]
    fn entity_level();
}
