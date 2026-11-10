//! Owner-created rotational throws, distinct from generic entity summoning.
#[cfg(test)]
#[path = "native_projectile_launch_tests.rs"]
mod tests;
use super::Game;
use horizon_client::{
    items::ItemStack,
    projectiles::{Projectile, ProjectileKind},
};
use horizon_core::geometry::Vec3;
use mod_api::{Identifier, LivingRef, ProjectileRef};

impl Game<'_> {
    pub(super) fn spawn_projectile_from_rotation(
        &mut self,
        owner: LivingRef,
        entity_type: &Identifier,
        stack: &ItemStack,
        offset: f32,
        power: f32,
        inaccuracy: f32,
    ) -> Option<ProjectileRef> {
        if self.remote.is_some() || ![offset, power, inaccuracy].into_iter().all(f32::is_finite) {
            return None;
        }
        self.native_living_target(owner)?;
        let kind = match entity_type.to_string().as_str() {
            "minecraft:snowball" => ProjectileKind::Snowball,
            "minecraft:egg" => ProjectileKind::Egg,
            "minecraft:ender_pearl" => ProjectileKind::EnderPearl,
            "minecraft:experience_bottle" => ProjectileKind::ExperienceBottle,
            "minecraft:wind_charge" => ProjectileKind::WindCharge,
            "minecraft:splash_potion" => ProjectileKind::Potion(Box::new(stack.clone()), false),
            "minecraft:lingering_potion" => ProjectileKind::Potion(Box::new(stack.clone()), true),
            id => ProjectileKind::Thrown(horizon_client::projectiles::thrown_type(id)?),
        };
        let entity = owner.entity();
        let [x, y, z] = self.entity_eye_position(entity)?;
        let [yaw, pitch] = self.entity_rotation(entity)?;
        let motion = self.native_entity_known_speed(entity)?;
        let on_ground = self.entity_on_ground(entity)?;
        let ((who, uuid, guest), vehicle) = self.native_projectile_owner_fields(Some(entity))?;
        let mut shot = Projectile::new(kind, Vec3::new(x, y - 0.1, z), who);
        (shot.owner_uuid, shot.guest, shot.owner_vehicle) = (uuid, guest, vehicle);
        // Splitting the random from the record avoids borrowing it and the shot
        // simultaneously; cloning an RNG draws nothing and the advanced copy is restored.
        let mut random = shot.random.0.clone();
        shot.shoot_from_rotation(
            &mut random,
            pitch,
            yaw,
            offset,
            power,
            inaccuracy,
            motion,
            on_ground,
        );
        shot.random.0 = random;
        shot.item = Some(stack.clone());
        self.next_entity += 1;
        shot.id = self.next_entity;
        let id = shot.id;
        self.add_projectile(shot);
        self.projectile(self.entity(id)?)
    }
}
