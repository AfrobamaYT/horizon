//! An entity's invulnerability for native server mods, through an
//! `EntityRef` (docs/native-mods.md, "Entity and level handles"):
//! `Entity.invulnerable`, the flag `/data` and the `Invulnerable` tag set,
//! and `LivingEntity.invulnerableTime`, the ticks after a hit in which a
//! weaker one does nothing. Reads return None and writes false for a stale
//! handle, on a client, and for a kind Horizon keeps no such state for.
use super::{Game, commands::Target};
use horizon_client::mod_api::EntityRef;

impl Game<'_> {
    /// A player's body, where it is kept now.
    fn native_player_body(&self, t: Target) -> Option<&horizon_client::motion::PlayerMotion> {
        match t {
            Target::Player => self.player.as_ref(),
            Target::Guest(conn) => self.lan.as_ref()?.hub.guest(conn)?.motion.as_deref(),
            _ => None,
        }
    }
    fn native_player_body_mut(
        &mut self,
        t: Target,
    ) -> Option<&mut horizon_client::motion::PlayerMotion> {
        match t {
            Target::Player => self.player.as_mut(),
            Target::Guest(conn) => self
                .lan
                .as_mut()?
                .hub
                .guest_mut(conn)?
                .motion
                .as_deref_mut(),
            _ => None,
        }
    }

    /// Whether the entity is invulnerable (`Entity.isInvulnerable`): only
    /// damage that bypasses invulnerability (the void, `/kill`) reaches it.
    /// Mobs keep the flag; a player's is always false in Horizon, which has
    /// no `Invulnerable` tag for players (creative's invulnerability is an
    /// ability, not this flag). API @API:entity_invulnerable@.
    pub(super) fn native_entity_invulnerable(&self, entity: EntityRef) -> Option<bool> {
        match self.native_entity_target(entity)? {
            Target::Mob(id) => Some(self.mobs.iter().find(|m| m.id == id)?.invulnerable_flag),
            t @ (Target::Player | Target::Guest(_)) => self.native_player_body(t).map(|_| false),
            _ => None,
        }
    }
    /// Sets a mob's invulnerable flag (`Entity.setInvulnerable`), saved
    /// with it as `Invulnerable`. False for the other kinds. API
    /// @API:entity_invulnerable@.
    pub(super) fn native_entity_set_invulnerable(
        &mut self,
        entity: EntityRef,
        invulnerable: bool,
    ) -> bool {
        let Some(Target::Mob(id)) = self.native_entity_target(entity) else {
            return false;
        };
        let Some(m) = self.mobs.iter_mut().find(|m| m.id == id) else {
            return false;
        };
        m.invulnerable_flag = invulnerable;
        true
    }
    /// `LivingEntity.invulnerableTime`: 20 after a hit, counting down; while
    /// it is above 10 a hit does only what it deals beyond the last one.
    /// Mobs and players. API @API:entity_invulnerable@.
    pub(super) fn native_entity_invulnerable_time(&self, entity: EntityRef) -> Option<i32> {
        match self.native_entity_target(entity)? {
            Target::Mob(id) => Some(self.mobs.iter().find(|m| m.id == id)?.invulnerable),
            t => Some(self.native_player_body(t)?.health.hurt.cooldown),
        }
    }
    /// Sets `LivingEntity.invulnerableTime` (`invulnerableTime = ticks`), as
    /// mods do to let rapid hits through (0) or to give a grace period.
    /// Mobs and players. API @API:entity_invulnerable@.
    pub(super) fn native_entity_set_invulnerable_time(
        &mut self,
        entity: EntityRef,
        ticks: i32,
    ) -> bool {
        let Some(t) = self.native_entity_target(entity) else {
            return false;
        };
        match t {
            Target::Mob(id) => match self.mobs.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.invulnerable = ticks;
                    true
                }
                None => false,
            },
            t => match self.native_player_body_mut(t) {
                Some(p) => {
                    p.health.hurt.cooldown = ticks;
                    true
                }
                None => false,
            },
        }
    }
}
