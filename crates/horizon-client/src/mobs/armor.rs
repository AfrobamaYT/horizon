//! Effective mob armor and toughness at the damage boundary.
use super::Mob;
use horizon_world::attributes::{Definition, entity::EntityAttributes};

fn effective(map: Option<&EntityAttributes>, name: &str, base: f64, maximum: f64) -> f64 {
    if let Some(map) = map
        && let Some(key) = map.catalog().id(name)
    {
        if let Some(source) = map.instance(key) {
            return source.value();
        }
        let definition = map.catalog().attribute(key).definition;
        return definition.sanitize(map.base_value(key).unwrap_or(base));
    }
    Definition::ranged(0.0, 0.0, maximum)
        .unwrap()
        .sanitize(base)
}

/// LivingEntity.getArmorValue floors the effective double before converting
/// its integer result to float. Toughness converts its effective double directly.
///
/// The live attribute map answers first (#941): the type's base, what
/// `/attribute` and the save left, a mod's modifiers, and every worn or held
/// stack's own `attribute_modifiers`, components included — the same map a
/// `LivingRef` reads, which the tick's equipment pass keeps current. A
/// shulker's `minecraft:covered` lives on that map too. Without a map yet
/// (nothing equipped, nothing written), the type's own armor fields and the
/// `/attribute` armor map answer, as before.
pub fn values(mob: &Mob) -> (f32, f32) {
    if let Some(live) = mob.extra.live.built() {
        let toughness = live.key("minecraft:armor_toughness").and_then(|key| {
            live.value(Some(key), horizon_world::attribute_map::ValueKind::Computed)
                .ok()
        });
        let armor = live.key("minecraft:armor").and_then(|key| {
            live.value(Some(key), horizon_world::attribute_map::ValueKind::Computed)
                .ok()
        });
        if let (Some(armor), Some(toughness)) = (armor, toughness) {
            return (
                armor.clamp(0.0, 30.0).floor() as i32 as f32,
                toughness.clamp(0.0, 20.0) as f32,
            );
        }
    }
    let map = mob.extra.armor_attributes.as_deref();
    let armor = effective(map, "minecraft:armor", f64::from(mob.kind.armor), 30.0);
    let toughness = effective(
        map,
        "minecraft:armor_toughness",
        f64::from(mob.kind.armor_toughness),
        20.0,
    );
    (armor.floor() as i32 as f32, toughness as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::{Damage, DamageKind, Species};
    use horizon_core::geometry::Vec3;
    use horizon_world::attributes::{Modifier, Operation, entity::Catalog};
    use std::sync::Arc;

    /// A pig whose live attribute map holds `armor` and `toughness` bases, as
    /// `/attribute` writes and the save leave them, with its catalog for edits.
    fn pig(armor: f64, toughness: f64) -> (Mob, Arc<Catalog>) {
        let bits = |v: f64| format!("{:016x}", v.to_bits());
        let catalog = Arc::new(Catalog::from_json(&serde_json::json!({
            "data_version": horizon_world::DATA_VERSION,
            "attributes": [
                {"id":"minecraft:armor", "default":bits(0.0), "min":bits(0.0), "max":bits(30.0), "syncable":true},
                {"id":"minecraft:armor_toughness", "default":bits(0.0), "min":bits(0.0), "max":bits(20.0), "syncable":true}
            ],
            "entities": {"minecraft:pig":{"minecraft:armor":bits(0.0), "minecraft:armor_toughness":bits(0.0)}}
        })).unwrap());
        let mut live = crate::living_attributes::LivingAttributes::default();
        for (name, base) in [
            ("minecraft:armor", armor),
            ("minecraft:armor_toughness", toughness),
        ] {
            live.edit(
                &catalog,
                "minecraft:pig",
                name,
                true,
                Vec::new,
                |value, observer| value.set_base_value(base, observer),
            )
            .unwrap();
        }
        let mut kind = crate::mobs::sensor_scan_tests::kind();
        let ty = Arc::make_mut(&mut kind);
        ty.id = "minecraft:pig".into();
        ty.species = Species::Pig;
        let mut mob = Mob::new(1, kind, Vec3::ZERO, 0.0);
        mob.extra.live = live;
        mob.attrs.max_health = 1000.0;
        mob.health = 1000.0;
        (mob, catalog)
    }

    #[test]
    fn effective_mob_armor_controls_damage_and_fractional_armor_floors() {
        let mut test_level = crate::health::TestLevel::new(0);
        let level = &mut test_level.level();
        for (armor, toughness, source, expected) in [
            (20.0, 8.0, DamageKind::Cactus, 999039),
            (0.0, 8.0, DamageKind::Cactus, 996000),
            (0.5, 8.0, DamageKind::Cactus, 996000),
            (20.0, 8.0, DamageKind::SonicBoom, 996000),
        ] {
            let (mut mob, _) = pig(armor, toughness);
            assert!(mob.damage(level, 4.0, Damage::of(source)));
            assert_eq!((f64::from(mob.health) * 1000.0) as i32, expected);
        }
    }

    /// A worn stack's modifiers are the live map's transient rows, whatever
    /// changed them: a mod's, `/item`'s or `set_equipment`'s (#941).
    fn wear(mob: &mut Mob, catalog: &Arc<Catalog>, amount: f64, operation: Operation) {
        mob.extra
            .live
            .edit(
                catalog,
                "minecraft:pig",
                "minecraft:armor",
                true,
                Vec::new,
                |value, observer| {
                    value.add_transient(
                        Modifier::new("minecraft:armor.helmet", amount, operation).unwrap(),
                        observer,
                    )
                },
            )
            .unwrap();
    }

    #[test]
    fn equipment_is_counted_once_before_effective_limits() {
        let (mut mob, catalog) = pig(20.0, 8.0);
        wear(&mut mob, &catalog, 3.0, Operation::AddValue);
        assert_eq!(values(&mob), (23.0, 8.0));
        let (mut mob, catalog) = pig(10.0, 8.0);
        wear(&mut mob, &catalog, 0.5, Operation::AddMultipliedTotal);
        assert_eq!(values(&mob), (15.0, 8.0));
        let (mob, _) = pig(100.0, 100.0);
        assert_eq!(values(&mob), (30.0, 20.0));
    }

    /// A mob that never built a map answers from its type's armor fields, and
    /// one whose type lacks the attributes from the range's floor.
    #[test]
    fn a_mob_without_a_map_answers_its_types_fields() {
        let mut kind = crate::mobs::sensor_scan_tests::kind();
        let ty = Arc::make_mut(&mut kind);
        ty.armor = 7.0;
        ty.armor_toughness = 2.0;
        let mob = Mob::new(1, kind, Vec3::ZERO, 0.0);
        assert_eq!(values(&mob), (7.0, 2.0));
        assert!(mob.extra.live.built().is_none());
    }
}
