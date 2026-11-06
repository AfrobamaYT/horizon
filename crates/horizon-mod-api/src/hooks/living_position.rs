//! The `living_position` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_position::Game<'_>::native_living_has_line_of_sight"
    )]
    fn living_has_line_of_sight();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_position::Game<'_>::native_living_random_teleport"
    )]
    fn living_random_teleport();
    #[notes = r#"## The box of a pose

`native_living_dimensions_for_pose(living, pose)` answers what vanilla's `LivingEntity.getDimensions(Pose)` does: the box (width, height and eye height) the living entity has in the pose asked for, its `minecraft:scale` included. The fixed 0.2 box of a sleeping pose is the same for every species, and the species' own state still answers where its `getDefaultDimensions` reads it — a digging sniffer or an emerging warden is low in every pose. `Pose` (`mod_api::Pose`, vanilla's network set) names the stance."#]
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_position::Game<'_>::native_living_dimensions_for_pose"
    )]
    fn living_dimensions_for_pose();
}
