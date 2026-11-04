//! The `entity_motion` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_velocity"
    )]
    fn entity_velocity();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_set_velocity"
    )]
    fn entity_set_velocity();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_on_ground"
    )]
    fn entity_on_ground();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_bounding_box"
    )]
    fn entity_bounding_box();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_eye_height"
    )]
    fn entity_eye_height();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_eye_position"
    )]
    fn entity_eye_position();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_view_vector"
    )]
    fn entity_view_vector();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_motion::Game<'_>::native_entity_distance_to_sqr"
    )]
    fn entity_distance_to_sqr();
}
