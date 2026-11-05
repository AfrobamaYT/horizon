//! Kept block geometry's time-based rotation, on the generated drawing resource.

pub trait Helpers {
    /// What is drawn next turns by itself: `degrees_per_tick` degrees each
    /// game tick (a shaft at `rpm` turns `rpm * 0.3`, as Create's do), about
    /// `axis` through `pivot`, both in blocks from the block's corner
    /// whatever the pose, counterclockwise seen from the axis' positive end,
    /// `phase` degrees round at game time 0 (to mesh a cog with its
    /// neighbour's). Draw the part at rest; the game turns it each frame from
    /// the game time and the partial tick. With `keep` it goes on turning
    /// with no call of `native_block_entity_submit` until what it was drawn
    /// with changes: send a new speed in the update tag and the block is
    /// drawn again (Flywheel's rotating instances, done on the CPU). The
    /// angle is `game_time * degrees_per_tick + phase`, not a sum of turns:
    /// with a new speed, pass the phase that keeps the part where it was
    /// (`old_angle - game_time * new_speed`), or it jumps. Until
    /// `still` or the next `spin`. A block entity's frame only: a mob's
    /// (`native_mob_submit`) draws it at rest. API @API:spinning_geometry@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_spin"
    )]
    fn draw_spin();
    /// What is drawn next stands still again, after `spin`.
    /// API @API:spinning_geometry@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_still"
    )]
    fn draw_still();
}
