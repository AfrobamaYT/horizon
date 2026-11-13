//! The player's collision exclusions (#3717) in its collision checks: `Entity.collide` reads them
//! in `MotionWorld::collide`, `Level.noCollision` here.
use super::*;
use horizon_world::world_movement::CollisionCheckRequest;

impl MotionWorld<'_, '_, '_> {
    /// `Level.noCollision(player, bounds)`: blocks, less the player's collision exclusions, and
    /// the host's entity boxes.
    pub fn no_player_collision(&mut self, bounds: Aabb) -> Result<bool> {
        let context = self.context();
        Ok(collision_exclusions::no_world_collision(
            &mut self.movement_collisions(),
            &self.player.collision_exclusions,
            CollisionCheckRequest {
                entity: EntityIdentity(0),
                context,
                bounds,
                candidate_budget: 1_000_000,
            },
        )?)
    }
}
