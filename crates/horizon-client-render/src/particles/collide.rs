//! A particle's own collision step, in its own module so `particles.rs`
//! keeps its size.
use super::ParticleLevel;
use crate::collisions::SceneCollisions;
use horizon_core::geometry::{Aabb, Vec3};
impl ParticleLevel<'_> {
    pub(super) fn collide(&self, bb: Aabb, d: [f64; 3]) -> [f64; 3] {
        let mut scene = SceneCollisions {
            world: self.world,
            binding: self.collision,
            subject: None,
        };
        let movement = Vec3::new(d[0], d[1], d[2]);
        if scene.cells_without_collision(bb.expand_towards(movement)) {
            return d;
        }
        match scene.resolve(bb, movement, 0.0, false) {
            Ok(v) => [v.x, v.y, v.z],
            Err(_) => d,
        }
    }
}
