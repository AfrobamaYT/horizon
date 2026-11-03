//! A living entity's active effects for native server mods, through a
//! `LivingRef` (docs/native-mods.md, "Entity and level handles"): vanilla's
//! `activeEffects` read whole, and `addEffect`, `removeEffect`,
//! `removeAllEffects` and `hasEffect` by the game's own paths, so an effect
//! a mod gives lands as a potion's or `/effect`'s does, through
//! `canBeAffected` and the added-effect hooks. Reads return `None`, writes
//! false, for a stale handle or on a client; armor stands and the dragon
//! keep no effects, as `/effect` skips them.
use super::Game;
use super::commands::Target;
use super::potions::mob_can_be_affected;
use horizon_client::brewing;
use horizon_client::effects::{EffectInstance, PlayerEffects};
use horizon_client::mobs::Mob;
use horizon_client::mod_api::{LivingRef, NativeActiveEffect};

/// A player's effect as the API's: the id namespaced as the registry has it.
fn active_effect(e: &EffectInstance) -> NativeActiveEffect {
    NativeActiveEffect {
        id: brewing::namespaced(e.id),
        duration: e.duration,
        amplifier: e.amplifier,
        ambient: e.ambient,
        visible: e.visible,
    }
}

/// A mob's effect as the API's: mobs keep the effect itself as
/// (id, ticks, amplifier), the ambient and particle flags in their own lists.
fn mob_effect(m: &Mob, e: &(&'static str, i32, i32)) -> NativeActiveEffect {
    NativeActiveEffect {
        id: e.0,
        duration: e.1,
        amplifier: e.2,
        ambient: m.extra.ambient_effects.contains(&e.0),
        visible: !m.extra.hidden_effects.contains(&e.0),
    }
}

impl Game<'_> {
    /// Where a player's or a guest's effects are: the acting player's own
    /// are the game's (swapped in by `enter_guest`; a player handle names
    /// the acting player, and a game without a hub keeps its single
    /// player's the same way), any other guest's are their hub record's.
    /// None for another kind.
    fn native_living_player_effects(&self, t: Target) -> Option<&PlayerEffects> {
        match t {
            Target::Player => Some(&self.potions.effects),
            Target::Guest(conn) if self.lan_acting == Some(conn) => Some(&self.potions.effects),
            Target::Guest(conn) => Some(&self.lan.as_ref()?.hub.guest(conn)?.effects),
            _ => None,
        }
    }

    /// `LivingEntity.addEffect(instance)`: gives the living entity the
    /// effect by the game's own path. A player's or a guest's goes through
    /// `add_player_effect` (so `MobEffectInstance.update`'s rules, the
    /// attribute modifiers, absorption and the added-effect hooks of a
    /// content mod's effect all apply); a mob's goes past `canBeAffected`
    /// (a spider shrugs off poison) into its effects with the same hooks.
    /// The amplifier is cut to 0..=255 and the duration is -1 (infinite) or
    /// ticks. False for an unknown effect id, a duration below -1, an armor
    /// stand, the dragon, a stale handle or a client. True means accepted,
    /// as vanilla's `addEffect` does: whether it replaced one already there
    /// or only extended it is `MobEffectInstance.update`'s business.
    pub(super) fn native_living_add_effect(
        &mut self,
        living: LivingRef,
        effect: &NativeActiveEffect,
    ) -> bool {
        let Some(def) = brewing::effect(effect.id) else {
            return false;
        };
        if effect.duration < -1 {
            return false;
        }
        let amplifier = effect.amplifier.clamp(0, 255);
        match self.native_living_target(living) {
            Some(t @ (Target::Player | Target::Guest(_))) => {
                let mut instance = EffectInstance::new(def.id, effect.duration, amplifier);
                instance.ambient = effect.ambient;
                instance.visible = effect.visible;
                self.for_player(t, |g| g.add_player_effect(instance))
                    .is_some()
            }
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                let kind = m.kind.id.clone();
                let name = brewing::namespaced(def.id);
                let new = !m.has_effect(name);
                if !mob_can_be_affected(&self.res.stations().enchantments.entity_tags, &kind, name)
                {
                    return false;
                }
                let Some(m) = self.mobs.iter_mut().find(|m| m.id == id) else {
                    return false;
                };
                m.add_effect_with(
                    name,
                    effect.duration,
                    amplifier,
                    effect.ambient,
                    effect.visible,
                );
                self.mod_effect_added(name, Some(id as i32), amplifier, new);
                true
            }
            _ => false,
        }
    }

    /// The entity's instance of `effect` (`LivingEntity.getEffect`): its id
    /// namespaced as the registry has it, the ticks left (-1 infinite), the
    /// amplifier and the flags. A bare vanilla id reads as `minecraft:`'s.
    /// None when it has the effect not, or for an unknown id, an armor
    /// stand, the dragon, a stale handle or a client.
    pub(super) fn native_living_effect(
        &self,
        living: LivingRef,
        effect: &str,
    ) -> Option<NativeActiveEffect> {
        let def = brewing::effect(effect)?;
        match self.native_living_target(living)? {
            t @ (Target::Player | Target::Guest(_)) => self
                .native_living_player_effects(t)?
                .get(def.id)
                .map(active_effect),
            Target::Mob(id) => {
                let m = self.mobs.iter().find(|m| m.id == id && !m.removed)?;
                let name = brewing::namespaced(def.id);
                let e = m.extra.effects.iter().find(|e| e.0 == name)?;
                Some(mob_effect(m, e))
            }
            _ => None,
        }
    }

    /// The entity's active effects (`LivingEntity.getActiveEffects`), in the
    /// order their store keeps them, in `Some` even when it has none. None
    /// for an armor stand, the dragon, a stale handle or a client.
    pub(super) fn native_living_effects(
        &self,
        living: LivingRef,
    ) -> Option<Vec<NativeActiveEffect>> {
        match self.native_living_target(living)? {
            t @ (Target::Player | Target::Guest(_)) => self
                .native_living_player_effects(t)
                .map(|effects| effects.list.iter().map(active_effect).collect()),
            Target::Mob(id) => self
                .mobs
                .iter()
                .find(|m| m.id == id && !m.removed)
                .map(|m| m.extra.effects.iter().map(|e| mob_effect(m, e)).collect()),
            _ => None,
        }
    }

    /// Whether the entity has `effect` (`LivingEntity.hasEffect`; Paper's
    /// `LivingEntity#hasPotionEffect`). A bare vanilla id reads as
    /// `minecraft:`'s. False for an unknown id, an armor stand, the dragon,
    /// a stale handle or a client.
    pub(super) fn native_living_potion_affected(&self, living: LivingRef, effect: &str) -> bool {
        let Some(def) = brewing::effect(effect) else {
            return false;
        };
        match self.native_living_target(living) {
            Some(t @ (Target::Player | Target::Guest(_))) => self
                .native_living_player_effects(t)
                .is_some_and(|effects| effects.get(def.id).is_some()),
            Some(Target::Mob(id)) => self
                .mobs
                .iter()
                .find(|m| m.id == id && !m.removed)
                .is_some_and(|m| m.has_effect(brewing::namespaced(def.id))),
            _ => false,
        }
    }

    /// `LivingEntity.removeEffect(effect)` the way its running out goes
    /// (`LivingEntity.tickEffects` drops it): what the effect gave the
    /// player goes with it — the attribute modifiers recompute
    /// (`apply_effect_status`) and absorption's hearts leave — and a mob's
    /// hidden and ambient lists drop it too, with absorption's hearts
    /// leaving the same way its expiry takes them. False (and nothing
    /// changes) when the entity has the effect not, or for an unknown id,
    /// an armor stand, the dragon, a stale handle or a client.
    pub(super) fn native_living_remove_effect(&mut self, living: LivingRef, effect: &str) -> bool {
        let Some(def) = brewing::effect(effect) else {
            return false;
        };
        match self.native_living_target(living) {
            Some(t @ (Target::Player | Target::Guest(_))) => self
                .for_player(t, |g| {
                    let before = g.potions.effects.list.len();
                    g.potions.effects.list.retain(|x| x.id != def.id);
                    g.apply_effect_status();
                    g.potions.effects.list.len() != before
                })
                .unwrap_or(false),
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter_mut().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                let name = brewing::namespaced(def.id);
                if !m.extra.effects.iter().any(|e| e.0 == name) {
                    return false;
                }
                native_mob_remove_effects(m, [name]);
                true
            }
            _ => false,
        }
    }

    /// `LivingEntity.removeAllEffects()` the way their running out goes:
    /// every effect's attribute modifiers and absorption hearts leave with
    /// them. False when the entity has none, or for an armor stand, the
    /// dragon, a stale handle or a client.
    pub(super) fn native_living_remove_all_effects(&mut self, living: LivingRef) -> bool {
        match self.native_living_target(living) {
            Some(t @ (Target::Player | Target::Guest(_))) => self
                .for_player(t, |g| {
                    let any = !g.potions.effects.list.is_empty();
                    g.potions.effects.clear();
                    g.apply_effect_status();
                    any
                })
                .unwrap_or(false),
            Some(Target::Mob(id)) => {
                let Some(m) = self.mobs.iter_mut().find(|m| m.id == id && !m.removed) else {
                    return false;
                };
                let any = !m.extra.effects.is_empty();
                let names: Vec<&'static str> = m.extra.effects.iter().map(|e| e.0).collect();
                native_mob_remove_effects(m, names);
                any
            }
            _ => false,
        }
    }
}

/// A mob's effects taken away the way their expiry takes them
/// (`Mob.tickEffects`): absorption's hearts go with theirs
/// (`AbsorptionMobEffect.removeAttributeModifiers`), the effects themselves
/// drop, and the hidden and ambient lists keep only what stays.
fn native_mob_remove_effects(m: &mut Mob, names: impl IntoIterator<Item = &'static str>) {
    let names: Vec<&'static str> = names.into_iter().collect();
    for name in &names {
        if *name == "minecraft:absorption"
            && let Some((_, _, amp)) = m.extra.effects.iter().find(|e| e.0 == *name)
        {
            m.absorption = (m.absorption - 4.0 * (amp + 1) as f32).max(0.0);
        }
    }
    m.extra.effects.retain(|x| !names.contains(&x.0));
    let left = &m.extra.effects;
    m.extra
        .hidden_effects
        .retain(|h| left.iter().any(|x| x.0 == *h));
    m.extra
        .ambient_effects
        .retain(|h| left.iter().any(|x| x.0 == *h));
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use horizon_client::mobs::Mob;
    use horizon_client::mod_api::{EntityRef, LivingRef, NativeActiveEffect, NativeWorldEpoch};
    use horizon_core::geometry::Vec3;

    fn effect(id: &'static str, duration: i32, amplifier: i32) -> NativeActiveEffect {
        NativeActiveEffect {
            id,
            duration,
            amplifier,
            ambient: false,
            visible: true,
        }
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_mob_takes_effects_by_the_games_own_paths() {
        with_game(|game| {
            for (id, kind) in [
                (12, "minecraft:zombie"),
                (13, "minecraft:spider"),
                (14, "minecraft:cow"),
            ] {
                let kind = game.res.mob_types().get(kind).unwrap().clone();
                game.mobs.push(Mob::new(id, kind, Vec3::ZERO, 0.0));
            }
            let zombie = game.native_living(game.native_entity(12).unwrap()).unwrap();
            let spider = game.native_living(game.native_entity(13).unwrap()).unwrap();
            let cow = game.native_living(game.native_entity(14).unwrap()).unwrap();

            // add_effect gives it, and the bare spelling reads the same.
            assert!(game.native_living_add_effect(zombie, &effect("minecraft:speed", 200, 0)));
            assert!(game.native_living_potion_affected(zombie, "speed"));
            assert_eq!(
                game.native_living_effect(zombie, "speed"),
                Some(effect("minecraft:speed", 200, 0))
            );
            assert_eq!(
                game.native_living_effects(zombie),
                Some(vec![effect("minecraft:speed", 200, 0)])
            );

            // A re-add is `MobEffectInstance.update`: a weaker or shorter
            // one changes nothing, a stronger replaces with its own
            // duration, an equal one only extends.
            assert!(game.native_living_add_effect(zombie, &effect("minecraft:speed", 100, 0)));
            assert_eq!(
                game.native_living_effect(zombie, "minecraft:speed"),
                Some(effect("minecraft:speed", 200, 0))
            );
            assert!(game.native_living_add_effect(zombie, &effect("minecraft:speed", 90, 1)));
            assert_eq!(
                game.native_living_effect(zombie, "minecraft:speed"),
                Some(effect("minecraft:speed", 90, 1))
            );

            // canBeAffected: a spider shrugs off poison and an undead one
            // with it, a cow takes it.
            assert!(!game.native_living_add_effect(spider, &effect("minecraft:poison", 100, 0)));
            assert_eq!(game.native_living_effects(spider), Some(Vec::new()));
            assert!(!game.native_living_add_effect(zombie, &effect("minecraft:poison", 100, 0)));
            assert!(game.native_living_add_effect(cow, &effect("minecraft:poison", 100, 0)));

            // An unknown id and a duration below -1 change nothing.
            assert!(!game.native_living_add_effect(zombie, &effect("minecraft:bogus", 100, 0)));
            assert!(!game.native_living_add_effect(zombie, &effect("minecraft:speed", -2, 0)));

            // remove_effect takes its absorption hearts with it; a second
            // remove has nothing to do.
            assert!(game.native_living_add_effect(zombie, &effect("minecraft:absorption", 100, 1)));
            game.mobs[0].absorption = 8.0;
            assert!(game.native_living_remove_effect(zombie, "minecraft:absorption"));
            assert_eq!(game.mobs[0].absorption, 0.0);
            assert!(!game.native_living_remove_effect(zombie, "minecraft:absorption"));
            assert!(game.native_living_potion_affected(zombie, "minecraft:speed"));

            // remove_all_effects takes the rest, and says when there was none.
            assert!(game.native_living_remove_all_effects(zombie));
            assert_eq!(game.native_living_effects(zombie), Some(Vec::new()));
            assert!(!game.native_living_potion_affected(zombie, "minecraft:speed"));
            assert!(!game.native_living_remove_all_effects(zombie));

            // A stale handle reads nothing and writes nothing.
            let stale =
                EntityRef::new(NativeWorldEpoch::new(zombie.entity().epoch().get() + 1), 12);
            let stale = LivingRef::new(stale);
            assert!(!game.native_living_add_effect(stale, &effect("minecraft:speed", 100, 0)));
            assert_eq!(game.native_living_effect(stale, "minecraft:speed"), None);
            assert_eq!(game.native_living_effects(stale), None);
            assert!(!game.native_living_potion_affected(stale, "minecraft:speed"));
            assert!(!game.native_living_remove_effect(stale, "minecraft:speed"));
            assert!(!game.native_living_remove_all_effects(stale));
        });
    }
}
