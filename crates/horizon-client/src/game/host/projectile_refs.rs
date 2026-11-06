//! The game's side of the `projectile_refs` area (`horizon-mod-api/src/host/projectile_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ProjectileRefsServerHost for Game<'_> {
    fn projectile(&self, entity: EntityRef) -> Option<ProjectileRef> {
        mod_api::__private::check_access(
            "projectile",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_ref(entity)
    }
}
impl ProjectileRefsBackend for Game<'_> {
    fn projectile_owner(&self, projectile: ProjectileRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "projectile_owner",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_owner(projectile)
    }
    fn projectile_pickup(&self, projectile: ProjectileRef) -> Option<NativeArrowPickup> {
        mod_api::__private::check_access(
            "projectile_pickup",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_pickup(projectile)
    }
    fn projectile_pierce_level(
        &self,
        projectile: ProjectileRef,
    ) -> Result<i8, PierceLevelError> {
        mod_api::__private::check_access(
            "projectile_pierce_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_pierce_level(projectile)
    }
    fn projectile_base_damage(&self, projectile: ProjectileRef) -> Option<f64> {
        mod_api::__private::check_access(
            "projectile_base_damage",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_base_damage(projectile)
    }
    fn projectile_critical(&self, projectile: ProjectileRef) -> Option<bool> {
        mod_api::__private::check_access(
            "projectile_critical",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_critical(projectile)
    }
}
impl ProjectileRefsBackendMut for Game<'_> {
    fn projectile_set_owner(
        &mut self,
        projectile: ProjectileRef,
        owner: Option<EntityRef>,
    ) -> bool {
        mod_api::__private::check_access(
            "projectile_set_owner",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_set_owner(projectile, owner)
    }
    fn projectile_deflect(
        &mut self,
        projectile: ProjectileRef,
        deflection: NativeDeflection,
        by: Option<EntityRef>,
        new_owner: Option<EntityRef>,
        by_attack: bool,
        power: [f64; 3],
    ) -> bool {
        mod_api::__private::check_access(
            "projectile_deflect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_deflect(
            projectile,
            deflection,
            by,
            new_owner,
            by_attack,
            power,
        )
    }
    fn projectile_set_pickup(
        &mut self,
        projectile: ProjectileRef,
        pickup: NativeArrowPickup,
    ) -> bool {
        mod_api::__private::check_access(
            "projectile_set_pickup",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_set_pickup(projectile, pickup)
    }
    fn projectile_set_pierce_level(
        &mut self,
        projectile: ProjectileRef,
        level: i8,
    ) -> Result<(), PierceLevelError> {
        mod_api::__private::check_access(
            "projectile_set_pierce_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_set_pierce_level(projectile, level)
    }
    fn projectile_set_base_damage(
        &mut self,
        projectile: ProjectileRef,
        damage: f64,
    ) -> bool {
        mod_api::__private::check_access(
            "projectile_set_base_damage",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_set_base_damage(projectile, damage)
    }
    fn projectile_set_critical(
        &mut self,
        projectile: ProjectileRef,
        critical: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "projectile_set_critical",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_projectile_set_critical(projectile, critical)
    }
}
