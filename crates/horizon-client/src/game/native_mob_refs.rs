//! Mob handles for native server mods.
use super::{Game, commands::Target};
use horizon_client::{
    mobs::Who,
    mod_api::{EntityRef, LivingRef, MobRef},
};

impl Game<'_> {
    /// The entity as a vanilla Mob. Players, armor stands, stale handles and
    /// clients are rejected. The dragon is a vanilla Mob, but is not yet
    /// supported (a #29 follow-up). Mobs taken out for ticking are unavailable
    /// (also a #29 follow-up). API @API:mob_refs@.
    pub(super) fn native_mob_ref(&self, entity: EntityRef) -> Option<MobRef> {
        match self.native_entity_target(entity)? {
            Target::Mob(_) => Some(MobRef::new(entity)),
            _ => None,
        }
    }

    /// `Mob.getTarget`. An unavailable mob or player binding reads as None;
    /// targets not representable as living handles also read as None.
    /// API @API:mob_refs@.
    pub(super) fn native_mob_target(&self, mob: MobRef) -> Option<LivingRef> {
        let Target::Mob(id) = self.native_entity_target(mob.entity())? else {
            return None;
        };
        let target = self.mobs.iter().find(|m| m.id == id)?.ai.target?;
        let target_id = match target {
            Who::Mob(id) => id,
            Who::Player => {
                let host = self.lan.as_ref()?;
                let conn = host.mob_players.get(&id)?;
                u32::try_from(host.hub.guest(*conn)?.entity_id).ok()?
            }
        };
        self.native_living(self.native_entity(target_id)?)
    }

    /// `Mob.setTarget`; None clears it. Only mobs and players whose binding
    /// can be stored in the current level are supported; armor stands and the
    /// dragon are refused. False also means a stale handle, client, or an
    /// unavailable mob/player.
    /// API @API:mob_refs@.
    pub(super) fn native_mob_set_target(&mut self, mob: MobRef, target: Option<LivingRef>) -> bool {
        let Some(Target::Mob(id)) = self.native_entity_target(mob.entity()) else {
            return false;
        };
        let (who, player) = match target {
            None => (None, None),
            Some(target) => match self.native_entity_target(target.entity()) {
                Some(Target::Mob(id)) => (Some(Who::Mob(id)), None),
                Some(Target::Guest(conn)) if self.lan_acting_uuid.is_none() => {
                    if self
                        .lan
                        .as_ref()
                        .unwrap()
                        .hub
                        .guest(conn)
                        .unwrap()
                        .dimension
                        != self.dim.id()
                    {
                        return false;
                    }
                    (Some(Who::Player), Some(conn))
                }
                Some(Target::Player) => match self.lan_acting {
                    Some(conn) if self.lan.is_some() => (Some(Who::Player), Some(conn)),
                    _ => return false,
                },
                _ => return false,
            },
        };
        if let Some(conn) = player {
            let Some(host) = self.lan.as_mut() else {
                return false;
            };
            host.mob_players.insert(id, conn);
        }
        self.mobs.iter_mut().find(|m| m.id == id).unwrap().ai.target = who;
        true
    }

    /// `Mob.isAggressive`, the synced mob flag. API @API:mob_refs@.
    pub(super) fn native_mob_is_aggressive(&self, mob: MobRef) -> Option<bool> {
        let Target::Mob(id) = self.native_entity_target(mob.entity())? else {
            return None;
        };
        Some(self.mobs.iter().find(|m| m.id == id)?.ai.aggressive)
    }

    /// `Mob.setAggressive`. False for an unavailable mob. API @API:mob_refs@.
    pub(super) fn native_mob_set_aggressive(&mut self, mob: MobRef, value: bool) -> bool {
        let Some(Target::Mob(id)) = self.native_entity_target(mob.entity()) else {
            return false;
        };
        self.mobs
            .iter_mut()
            .find(|m| m.id == id)
            .unwrap()
            .ai
            .aggressive = value;
        true
    }

    /// `Mob.isPersistenceRequired`. API @API:mob_refs@.
    pub(super) fn native_mob_persistence_required(&self, mob: MobRef) -> Option<bool> {
        let Target::Mob(id) = self.native_entity_target(mob.entity())? else {
            return None;
        };
        Some(self.mobs.iter().find(|m| m.id == id)?.persistent)
    }

    /// `Mob.setPersistenceRequired`, which sets the flag and cannot clear it.
    /// False for an unavailable mob. API @API:mob_refs@.
    pub(super) fn native_mob_set_persistence_required(&mut self, mob: MobRef) -> bool {
        let Some(Target::Mob(id)) = self.native_entity_target(mob.entity()) else {
            return false;
        };
        self.mobs
            .iter_mut()
            .find(|m| m.id == id)
            .unwrap()
            .persistent = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use super::*;
    use horizon_client::{entities::ArmorStand, mobs::Mob, mod_api::NativeWorldEpoch};
    use horizon_core::geometry::Vec3;

    #[test]
    #[ignore = "needs prepared resources"]
    fn player_target_round_trips_through_the_mob_binding() {
        use horizon_client::lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig};
        use horizon_protocol::packets::configuration::ClientInformation;
        use std::sync::{Arc, atomic::AtomicI32, mpsc};

        with_game(|game| {
            let settings = super::super::lan::LanSettings {
                port: 0,
                guest_commands: false,
                force_game_mode: false,
            };
            let net = NetConfig {
                announce: false,
                ..NetConfig::lan(0, "MobRefTest".into(), 2)
            };
            let config = HubConfig {
                default_game_mode: 0,
                guest_commands: false,
                force_game_mode: false,
                hardcore: false,
                view_distance: 10,
                simulation_distance: 10,
                dimension: "minecraft:overworld".into(),
                allow_flight: false,
                seed: 0,
                max_players: 2,
                integrated: false,
                levels: Default::default(),
            };
            game.lan_host(settings, net, config).unwrap();
            let (tx, _rx) = mpsc::channel();
            let mut guest = Guest::new(
                Joined {
                    conn: 1,
                    uuid: 123,
                    name: "guest".into(),
                    properties: Vec::new(),
                    info: ClientInformation::new(8),
                    brand: String::new(),
                    sender: PlayerSender::new(tx, Arc::default()),
                    latency: Arc::new(AtomicI32::new(0)),
                    address: "127.0.0.1:1".parse().unwrap(),
                },
                20,
                [0.0; 3],
                8,
            );
            guest.placed = true;
            game.lan.as_mut().unwrap().hub.guests.push(guest);
            let zombie = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs.push(Mob::new(12, zombie, Vec3::ZERO, 0.0));
            let mob = game
                .native_mob_ref(game.native_entity(12).unwrap())
                .unwrap();
            let player = game.native_living(game.native_entity(20).unwrap()).unwrap();
            assert!(mob.set_target(game, Some(player)));
            assert_eq!(mob.target(game), Some(player));
            assert_eq!(game.lan.as_ref().unwrap().mob_players.get(&12), Some(&1));
            game.lan.as_mut().unwrap().hub.guests[0].dimension = "minecraft:the_nether".into();
            assert!(!mob.set_target(game, Some(player)));
            assert_eq!(game.mobs[0].ai.target, Some(Who::Player));
            game.lan.as_mut().unwrap().mob_players.clear();
            assert_eq!(mob.target(game), None);
            assert_eq!(game.mobs[0].ai.target, Some(Who::Player));
        });
    }

    #[test]
    #[ignore = "needs prepared resources"]
    fn mob_handles_read_write_and_refuse_unavailable_entities() {
        with_game(|game| {
            let zombie = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            game.mobs
                .push(Mob::new(12, zombie.clone(), Vec3::ZERO, 0.0));
            game.mobs.push(Mob::new(13, zombie, Vec3::ZERO, 0.0));
            game.armor_stands.push(ArmorStand::new(14, Vec3::ZERO, 0.0));
            let mut dragon = super::super::dragon::Dragon::new(Vec3::ZERO, 0.0, 0);
            dragon.id = 15;
            game.dragon = Some(dragon);
            assert!(
                game.native_mob_ref(game.native_entity(15).unwrap())
                    .is_none()
            );
            let entity = game.native_entity(12).unwrap();
            let mob = game.native_mob_ref(entity).unwrap();
            let other = game.native_living(game.native_entity(13).unwrap()).unwrap();
            let stand = game.native_living(game.native_entity(14).unwrap()).unwrap();
            assert!(game.native_mob_ref(stand.entity()).is_none());
            assert_eq!(mob.living().entity(), entity);
            assert_eq!(game.native_mob_is_aggressive(mob), Some(false));
            assert!(game.native_mob_set_aggressive(mob, true));
            assert_eq!(game.native_mob_is_aggressive(mob), Some(true));
            assert_eq!(game.native_mob_persistence_required(mob), Some(false));
            assert!(game.native_mob_set_persistence_required(mob));
            assert_eq!(game.native_mob_persistence_required(mob), Some(true));
            assert!(mob.set_target(game, Some(other)));
            assert_eq!(mob.target(game), Some(other));
            assert!(!game.native_mob_set_target(mob, Some(stand)));
            assert_eq!(game.native_mob_target(mob), Some(other));
            let stale = MobRef::new(EntityRef::new(
                NativeWorldEpoch::new(entity.epoch().get() + 1),
                12,
            ));
            assert!(game.native_mob_ref(stale.entity()).is_none());
            assert!(!game.native_mob_set_target(stale, None));
            assert_eq!(game.native_mob_is_aggressive(stale), None);
            let taken = std::mem::take(&mut game.mobs);
            assert_eq!(game.native_mob_target(mob), None);
            assert!(!game.native_mob_set_target(mob, None));
            assert_eq!(taken[0].ai.target, Some(Who::Mob(13)));
            game.mobs = taken;
            assert!(game.native_mob_set_target(mob, None));
            assert_eq!(game.native_mob_target(mob), None);
        });
    }
}
