//! Projectile handles for native server mods: owner, deflection, and an
//! arrow's pickup, damage and critical flag.
use super::{Game, commands::Target};
use horizon_client::{
    arrows::Pickup,
    lan::playerdata::{ints_uuid, uuid_ints},
    mobs::Who,
    mod_api::{EntityRef, NativeArrowPickup, NativeDeflection, ProjectileRef},
    projectiles::{Projectile, ProjectileKind},
};
use horizon_core::{geometry::Vec3, random::RandomSource};

/// `AbstractHurtingProjectile`: the kinds with an acceleration power.
fn hurting(kind: &ProjectileKind) -> bool {
    matches!(
        kind,
        ProjectileKind::LargeFireball { .. }
            | ProjectileKind::SmallFireball
            | ProjectileKind::WitherSkull { .. }
            | ProjectileKind::WindCharge
            | ProjectileKind::BreezeWindCharge
    )
}

/// A new owner as the projectile stores it: who, UUID and the guest's connection.
type OwnerFields = (Option<Who>, Option<[i32; 4]>, Option<u32>);

impl Game<'_> {
    fn native_projectile_index(&self, projectile: ProjectileRef) -> Option<usize> {
        let Target::Projectile(id) = self.native_entity_target(projectile.entity())? else {
            return None;
        };
        self.creatures.projectiles.iter().position(|p| p.id == id)
    }

    fn native_projectile(&self, projectile: ProjectileRef) -> Option<&Projectile> {
        self.creatures
            .projectiles
            .get(self.native_projectile_index(projectile)?)
    }

    /// Narrows an entity to a projectile Horizon keeps in its projectile list,
    /// or None for another kind, a stale handle or a client. A trident a player
    /// threw is not in that list, so it is None.
    pub(super) fn native_projectile_ref(&self, entity: EntityRef) -> Option<ProjectileRef> {
        let projectile = ProjectileRef::new(entity);
        self.native_projectile_index(projectile).map(|_| projectile)
    }

    /// `Projectile.getOwner`: the owner as a resolved entity. None for no
    /// owner, one that is gone or in another level, or a stale handle.
    pub(super) fn native_projectile_owner(&self, projectile: ProjectileRef) -> Option<EntityRef> {
        let uuid = self.native_projectile(projectile)?.owner_uuid?;
        let target = self.target_by_uuid(ints_uuid(uuid))?;
        if self.target_level(target) != self.dim {
            return None;
        }
        let id = u32::try_from(self.network_entity_id(target)?).ok()?;
        self.native_entity(id)
    }

    /// The owner fields for `Projectile.setOwner(owner)`. None for an owner
    /// that is stale or not a player or mob: Horizon keeps only those.
    fn native_projectile_owner_fields(&mut self, owner: Option<EntityRef>) -> Option<OwnerFields> {
        let Some(owner) = owner else {
            return Some((None, None, None));
        };
        let guest = match self.native_entity_target(owner)? {
            Target::Player => (Who::Player, self.lan_acting),
            Target::Guest(conn) => (Who::Player, Some(conn)),
            Target::Mob(id) => (Who::Mob(id), None),
            _ => return None,
        };
        let uuid = uuid_ints(self.native_entity_uuid(owner)?);
        Some((Some(guest.0), Some(uuid), guest.1))
    }

    /// `Projectile.setOwner`. False for a stale projectile, or an owner that is
    /// stale or neither a player nor a mob. Does not touch `leftOwner`.
    pub(super) fn native_projectile_set_owner(
        &mut self,
        projectile: ProjectileRef,
        owner: Option<EntityRef>,
    ) -> bool {
        let Some(index) = self.native_projectile_index(projectile) else {
            return false;
        };
        let Some((who, uuid, guest)) = self.native_projectile_owner_fields(owner) else {
            return false;
        };
        let p = &mut self.creatures.projectiles[index];
        (p.owner, p.owner_uuid, p.guest) = (who, uuid, guest);
        true
    }

    /// `Projectile.deflect(deflection, by, newOwner, byAttack, power)`, with
    /// `power` per axis. `REVERSE` draws one float for the turn from the
    /// projectile's own random (an arrow's) or the level's, as the breeze path
    /// does; `AIM_DEFLECT` and `MOMENTUM_DEFLECT` change nothing without `by`.
    /// The new owner is set, then `onDeflection`: a hurting projectile's
    /// acceleration power is 0.1 by an attack, else halved. False, and nothing
    /// changed, for a stale projectile, `by` or owner, or a non-finite power.
    pub(super) fn native_projectile_deflect(
        &mut self,
        projectile: ProjectileRef,
        deflection: NativeDeflection,
        by: Option<EntityRef>,
        new_owner: Option<EntityRef>,
        by_attack: bool,
        power: [f64; 3],
    ) -> bool {
        let Some(index) = self.native_projectile_index(projectile) else {
            return false;
        };
        let Some(owner) = self.native_projectile_owner_fields(new_owner) else {
            return false;
        };
        if !power.iter().all(|v| v.is_finite()) {
            return false;
        }
        let aim = match (deflection, by) {
            (NativeDeflection::Reverse, _) | (_, None) => None,
            (NativeDeflection::AimDeflect, Some(by)) => {
                let Some(look) = self.native_entity_view_vector(by) else {
                    return false;
                };
                Some(look)
            }
            (NativeDeflection::MomentumDeflect, Some(by)) => {
                let Some([x, y, z]) = self.native_entity_velocity(by) else {
                    return false;
                };
                let v = Vec3::new(x, y, z).normalize();
                Some([v.x, v.y, v.z])
            }
        };
        let jitter = if deflection == NativeDeflection::Reverse
            && self.creatures.projectiles[index].arrow.is_none()
        {
            Some(self.world.random.next_float())
        } else {
            None
        };
        let p = &mut self.creatures.projectiles[index];
        match deflection {
            NativeDeflection::Reverse => {
                let draw = match (&mut p.arrow, jitter) {
                    (Some(arrow), _) => arrow.random.next_float(),
                    (None, jitter) => jitter.unwrap_or_default(),
                };
                let rotation = 170.0 + draw * 20.0;
                let v = p.velocity;
                p.velocity = Vec3::new(
                    v.x * (-power[0] * 0.5),
                    v.y * (-power[1] * 0.5),
                    v.z * (-power[2] * 0.5),
                );
                p.yaw += rotation;
                p.previous_yaw += rotation;
                p.needs_sync = true;
            }
            _ => {
                if let Some(aim) = aim {
                    p.velocity = Vec3::new(aim[0] * power[0], aim[1] * power[1], aim[2] * power[2]);
                    p.needs_sync = true;
                }
            }
        }
        (p.owner, p.owner_uuid, p.guest) = owner;
        if hurting(&p.kind) {
            p.acceleration = if by_attack { 0.1 } else { p.acceleration * 0.5 };
        }
        true
    }

    /// `AbstractArrow.pickup`, for an arrow or a mob's trident.
    pub(super) fn native_projectile_pickup(
        &self,
        projectile: ProjectileRef,
    ) -> Option<NativeArrowPickup> {
        Some(
            match self.native_projectile(projectile)?.arrow.as_ref()?.pickup {
                Pickup::Disallowed => NativeArrowPickup::Disallowed,
                Pickup::Allowed => NativeArrowPickup::Allowed,
                Pickup::CreativeOnly => NativeArrowPickup::CreativeOnly,
            },
        )
    }

    /// Sets `AbstractArrow.pickup`. False for a stale handle or a projectile
    /// that is not an arrow or a mob's trident.
    pub(super) fn native_projectile_set_pickup(
        &mut self,
        projectile: ProjectileRef,
        pickup: NativeArrowPickup,
    ) -> bool {
        let Some(index) = self.native_projectile_index(projectile) else {
            return false;
        };
        let Some(arrow) = self.creatures.projectiles[index].arrow.as_mut() else {
            return false;
        };
        arrow.pickup = match pickup {
            NativeArrowPickup::Disallowed => Pickup::Disallowed,
            NativeArrowPickup::Allowed => Pickup::Allowed,
            NativeArrowPickup::CreativeOnly => Pickup::CreativeOnly,
        };
        true
    }

    /// `AbstractArrow.getBaseDamage`, for an arrow (a trident's is fixed).
    pub(super) fn native_projectile_base_damage(&self, projectile: ProjectileRef) -> Option<f64> {
        match self.native_projectile(projectile)?.kind {
            ProjectileKind::Arrow { base_damage, .. } => Some(base_damage),
            _ => None,
        }
    }

    /// `AbstractArrow.setBaseDamage`. False for a stale handle, a projectile
    /// that is not an arrow, or a value that is not finite.
    pub(super) fn native_projectile_set_base_damage(
        &mut self,
        projectile: ProjectileRef,
        damage: f64,
    ) -> bool {
        let Some(index) = self.native_projectile_index(projectile) else {
            return false;
        };
        let ProjectileKind::Arrow { base_damage, .. } = &mut self.creatures.projectiles[index].kind
        else {
            return false;
        };
        if !damage.is_finite() {
            return false;
        }
        *base_damage = damage;
        true
    }

    /// `AbstractArrow.isCritArrow`, for an arrow.
    pub(super) fn native_projectile_critical(&self, projectile: ProjectileRef) -> Option<bool> {
        let p = self.native_projectile(projectile)?;
        matches!(p.kind, ProjectileKind::Arrow { .. }).then(|| p.critical())
    }

    /// `AbstractArrow.setCritArrow`; clients see the flag at once through the
    /// entity data. False for a stale handle or a projectile that is not an arrow.
    pub(super) fn native_projectile_set_critical(
        &mut self,
        projectile: ProjectileRef,
        critical: bool,
    ) -> bool {
        let Some(index) = self.native_projectile_index(projectile) else {
            return false;
        };
        let p = &mut self.creatures.projectiles[index];
        if !matches!(p.kind, ProjectileKind::Arrow { .. }) {
            return false;
        }
        p.set_critical(critical);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use super::*;
    use horizon_client::{mobs::Mob, mod_api::NativeWorldEpoch};

    fn arrow(owner: Who) -> Projectile {
        let kind = ProjectileKind::Arrow {
            base_damage: 2.0,
            critical: false,
            effect: None,
        };
        let mut p = Projectile::new(kind, Vec3::new(0.0, 64.0, 0.0), Some(owner));
        p.velocity = Vec3::new(1.0, 2.0, 3.0);
        p
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn projectile_handles_read_and_write_owner_deflection_and_arrow_state() {
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            let kind = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs.push(Mob::new(99, kind, Vec3::ZERO, 0.0));
            let mob = game.native_entity(99).unwrap();
            game.add_projectile(arrow(Who::Mob(99)));
            let id = game.creatures.projectiles[0].id;
            let entity = game.native_entity(id).unwrap();
            assert!(game.native_projectile_ref(mob).is_none());
            let handle = game.native_projectile_ref(entity).unwrap();
            assert_eq!(handle.entity(), entity);

            assert_eq!(handle.owner(game), Some(mob));
            assert!(handle.set_owner(game, None));
            assert_eq!(handle.owner(game), None);
            assert_eq!(game.creatures.projectiles[0].owner, None);
            assert!(handle.set_owner(game, Some(mob)));
            assert_eq!(handle.owner(game), Some(mob));
            assert_eq!(game.creatures.projectiles[0].owner, Some(Who::Mob(99)));
            assert!(!handle.set_owner(game, Some(entity)));

            assert_eq!(handle.pickup(game), Some(NativeArrowPickup::Disallowed));
            assert!(handle.set_pickup(game, NativeArrowPickup::CreativeOnly));
            assert_eq!(
                game.creatures.projectiles[0].arrow.as_ref().unwrap().pickup,
                Pickup::CreativeOnly
            );
            assert_eq!(handle.base_damage(game), Some(2.0));
            assert!(handle.set_base_damage(game, 7.5));
            assert!(!handle.set_base_damage(game, f64::NAN));
            assert_eq!(handle.base_damage(game), Some(7.5));
            assert_eq!(handle.critical(game), Some(false));
            assert!(handle.set_critical(game, true));
            assert!(game.creatures.projectiles[0].critical());

            // REVERSE: one float from the arrow's random, then the scaled velocity.
            let draw = game.creatures.projectiles[0]
                .arrow
                .as_ref()
                .unwrap()
                .random
                .clone()
                .next_float();
            let yaw = game.creatures.projectiles[0].yaw;
            assert!(handle.deflect(game, NativeDeflection::Reverse, None, None, false, [0.2; 3]));
            let p = &game.creatures.projectiles[0];
            assert_eq!(
                p.velocity,
                Vec3::new(1.0 * (-0.2 * 0.5), 2.0 * (-0.2 * 0.5), 3.0 * (-0.2 * 0.5))
            );
            assert_eq!(p.yaw, yaw + (170.0 + draw * 20.0));
            assert_eq!(p.owner, None);
            assert!(p.needs_sync);

            // AIM_DEFLECT along the deflector's look, scaled by the power.
            let look = game.native_entity_view_vector(mob).unwrap();
            assert!(handle.deflect(
                game,
                NativeDeflection::AimDeflect,
                Some(mob),
                Some(mob),
                true,
                [2.0; 3]
            ));
            let p = &game.creatures.projectiles[0];
            assert_eq!(
                p.velocity,
                Vec3::new(look[0] * 2.0, look[1] * 2.0, look[2] * 2.0)
            );
            assert_eq!(p.owner, Some(Who::Mob(99)));

            // MOMENTUM_DEFLECT of a still mob has no direction.
            assert!(handle.deflect(
                game,
                NativeDeflection::MomentumDeflect,
                Some(mob),
                None,
                false,
                [1.0; 3]
            ));
            assert_eq!(game.creatures.projectiles[0].velocity, Vec3::ZERO);

            // A hurting projectile's acceleration: 0.1 by an attack, else halved.
            let mut ball = Projectile::fireball(
                ProjectileKind::LargeFireball { power: 1 },
                Vec3::ZERO,
                None,
                [1.0, 0.0, 0.0],
            );
            ball.acceleration = 0.1;
            game.add_projectile(ball);
            let ball_id = game.creatures.projectiles[1].id;
            let ball = game
                .native_projectile_ref(game.native_entity(ball_id).unwrap())
                .unwrap();
            assert_eq!(ball.pickup(game), None);
            assert_eq!(ball.base_damage(game), None);
            assert!(!ball.set_critical(game, true));
            assert!(ball.deflect(game, NativeDeflection::Reverse, None, None, false, [1.0; 3]));
            assert_eq!(game.creatures.projectiles[1].acceleration, 0.05);
            assert!(ball.deflect(game, NativeDeflection::Reverse, None, None, true, [1.0; 3]));
            assert_eq!(game.creatures.projectiles[1].acceleration, 0.1);

            let stale = ProjectileRef::new(EntityRef::new(
                NativeWorldEpoch::new(entity.epoch().get() + 1),
                entity.id(),
            ));
            assert_eq!(stale.owner(game), None);
            assert!(!stale.set_pickup(game, NativeArrowPickup::Allowed));
            assert!(!stale.deflect(game, NativeDeflection::Reverse, None, None, false, [1.0; 3]));
            let taken = std::mem::take(&mut game.creatures.projectiles);
            assert!(handle.base_damage(game).is_none());
            game.creatures.projectiles = taken;
        });
    }
}
