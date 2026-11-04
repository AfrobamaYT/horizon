//! The item modifiers of the live attribute maps of mobs, armor stands and the
//! dragon (`LivingEntity.collectEquipmentChanges`). The map and the NBT are in
//! `horizon_client::living_attributes`.
use super::Game;

impl Game<'_> {
    /// `LivingEntity.tick`'s `collectEquipmentChanges` for what the server
    /// simulates besides players: each slot whose item is not the one the map
    /// holds swaps the old item's attribute modifiers for the new one's. An
    /// entity that changed nothing costs a compare of its eight slots, and
    /// one that wears nothing and never wore anything is skipped outright.
    pub(super) fn tick_equipment_attributes(&mut self) {
        if self.remote.is_some() || self.options.execution_role.is_client() {
            return;
        }
        let catalog = self.motion.attributes.clone();
        let items = world_items!(self);
        for m in self.mobs.iter_mut().filter(|m| !m.removed) {
            if m.extra.equipment.is_empty() && !m.extra.live.has_worn() {
                continue;
            }
            // The map is taken out so the seed can read the mob.
            let mut live = std::mem::take(&mut m.extra.live);
            let changed = live.collect_equipment_changes(
                &catalog,
                &m.kind.id,
                &m.extra.equipment,
                items,
                || {
                    horizon_client::entity_nbt::attribute_entries(m)
                        .into_iter()
                        .map(|(_, e)| e)
                        .collect()
                },
            );
            warn_failed(m.id, &m.kind.id, changed);
            m.extra.live = live;
        }
        for a in self.armor_stands.iter_mut() {
            if a.equipment.is_empty() && !a.live.has_worn() {
                continue;
            }
            let changed = a.live.collect_equipment_changes(
                &catalog,
                "minecraft:armor_stand",
                &a.equipment,
                items,
                Vec::new,
            );
            warn_failed(a.id, "minecraft:armor_stand", changed);
        }
        if let Some(d) = self.dragon.as_mut()
            && !(d.equipment.is_empty() && !d.live.has_worn())
        {
            let changed = d.live.collect_equipment_changes(
                &catalog,
                "minecraft:ender_dragon",
                &d.equipment,
                items,
                Vec::new,
            );
            warn_failed(d.id, "minecraft:ender_dragon", changed);
        }
    }
}

/// An item modifier the map refused (a bad modifier id) is logged, naming the
/// entity, instead of being lost.
fn warn_failed(id: u32, kind: &str, result: horizon_client::Result<()>) {
    if let Err(error) = result {
        log::warn!("equipment attributes of {kind} {id}: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::native_entity_refs::tests::with_game;
    use horizon_client::{entities::ArmorStand, items::ItemStack, mobs::Mob};
    use horizon_core::geometry::Vec3;
    use horizon_items::equipment::{CHEST, HEAD, MAINHAND};

    fn stack(item: &str) -> Option<ItemStack> {
        Some(ItemStack::new(item, 1))
    }

    /// A zombie, an armor stand and the dragon, ids 12, 13 and 14.
    fn spawn(game: &mut Game<'_>) {
        let zombie = game
            .res
            .mob_types()
            .get("minecraft:zombie")
            .unwrap()
            .clone();
        game.mobs.push(Mob::new(12, zombie, Vec3::ZERO, 0.0));
        game.armor_stands.push(ArmorStand::new(13, Vec3::ZERO, 0.0));
        let mut dragon = crate::game::dragon::Dragon::new(Vec3::ZERO, 0.0, 0);
        dragon.id = 14;
        game.dragon = Some(dragon);
    }

    fn read(game: &Game<'_>, id: u32, name: &str) -> (Option<f64>, Option<f64>) {
        let living = game.native_living(game.native_entity(id).unwrap()).unwrap();
        (
            living.attribute_value(game, name),
            living.attribute_base(game, name),
        )
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_zombie_reads_its_defaults() {
        with_game(|game| {
            spawn(game);
            assert_eq!(
                read(game, 12, "minecraft:max_health"),
                (Some(20.0), Some(20.0))
            );
            assert_eq!(
                read(game, 12, "minecraft:attack_damage"),
                (Some(3.0), Some(3.0))
            );
            // Not an attribute of an armor stand.
            assert_eq!(read(game, 13, "minecraft:attack_damage"), (None, None));
        });
    }

    /// A read of a mob that wears nothing builds its map once and keeps it,
    /// so the next reads allocate nothing.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_read_builds_the_map_once() {
        with_game(|game| {
            spawn(game);
            assert!(game.mobs[0].extra.live.built().is_none());
            read(game, 12, "minecraft:max_health");
            let first: *const _ = game.mobs[0].extra.live.built().unwrap();
            read(game, 12, "minecraft:attack_damage");
            read(game, 12, "minecraft:max_health");
            let again: *const _ = game.mobs[0].extra.live.built().unwrap();
            assert!(std::ptr::eq(first, again));
            // A changed base drops it; the next read builds it from the new base.
            game.mobs[0].extra.live.invalidate();
            assert!(game.mobs[0].extra.live.built().is_none());
            assert_eq!(
                read(game, 12, "minecraft:max_health"),
                (Some(20.0), Some(20.0))
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_held_sword_raises_the_attack_damage_until_it_is_put_away() {
        with_game(|game| {
            spawn(game);
            let damage = "minecraft:attack_damage";
            game.mobs[0].set_equipment_stack(MAINHAND, stack("minecraft:diamond_sword"));
            game.tick_equipment_attributes();
            // +6 from the sword's `base_attack_damage` modifier; the base stays.
            assert_eq!(read(game, 12, damage), (Some(9.0), Some(3.0)));
            // The same item again changes nothing (no duplicate modifier).
            game.tick_equipment_attributes();
            assert_eq!(read(game, 12, damage), (Some(9.0), Some(3.0)));
            // Another sword takes the first one's modifier off before its own.
            game.mobs[0].set_equipment_stack(MAINHAND, stack("minecraft:netherite_sword"));
            game.tick_equipment_attributes();
            assert_eq!(read(game, 12, damage), (Some(10.0), Some(3.0)));
            game.mobs[0].set_equipment_stack(MAINHAND, None);
            game.tick_equipment_attributes();
            assert_eq!(read(game, 12, damage), (Some(3.0), Some(3.0)));
            // A sword on the head is in no slot its modifiers name.
            game.mobs[0].set_equipment_stack(HEAD, stack("minecraft:diamond_sword"));
            game.tick_equipment_attributes();
            assert_eq!(read(game, 12, damage), (Some(3.0), Some(3.0)));
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_stand_and_the_dragon_wear_armor_modifiers() {
        with_game(|game| {
            spawn(game);
            let armor = "minecraft:armor";
            game.armor_stands[0]
                .equipment
                .set(HEAD, stack("minecraft:diamond_helmet"));
            game.dragon
                .as_mut()
                .unwrap()
                .equipment
                .set(CHEST, stack("minecraft:diamond_chestplate"));
            game.tick_equipment_attributes();
            assert_eq!(read(game, 13, armor), (Some(3.0), Some(0.0)));
            assert_eq!(read(game, 14, armor), (Some(8.0), Some(0.0)));
            game.armor_stands[0].equipment.set(HEAD, None);
            game.tick_equipment_attributes();
            assert_eq!(read(game, 13, armor), (Some(0.0), Some(0.0)));
            assert_eq!(read(game, 14, armor), (Some(8.0), Some(0.0)));
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_stale_handle_reads_none() {
        with_game(|game| {
            spawn(game);
            let living = game.native_living(game.native_entity(12).unwrap()).unwrap();
            assert_eq!(
                living.attribute_value(game, "minecraft:max_health"),
                Some(20.0)
            );
            game.mobs.clear();
            assert_eq!(living.attribute_value(game, "minecraft:max_health"), None);
            assert_eq!(living.attribute_base(game, "minecraft:max_health"), None);
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_saved_base_and_modifier_are_read_and_saved_again() {
        use horizon_world::nbt::Tag;
        with_game(|game| {
            spawn(game);
            let saved = Tag::List(vec![Tag::compound([
                ("id", Tag::String("minecraft:max_health".into())),
                ("base", Tag::Double(40.0f64.to_bits())),
                (
                    "modifiers",
                    Tag::List(vec![Tag::compound([
                        ("id", Tag::String("test:bonus".into())),
                        ("amount", Tag::Double(0.5f64.to_bits())),
                        ("operation", Tag::String("add_multiplied_base".into())),
                    ])]),
                ),
            ])]);
            game.armor_stands[0].live.load(Some(&saved), |_| true);
            assert_eq!(
                read(game, 13, "minecraft:max_health"),
                (Some(60.0), Some(40.0))
            );
            let mut list = Vec::new();
            game.armor_stands[0].live.save_into(&mut list);
            assert_eq!(Tag::List(list), saved);
        });
    }

    fn modifier(id: &str, amount: f64, operation: AttributeOperation) -> AttributeModifier {
        AttributeModifier::new(Identifier::parse(id).unwrap(), amount, operation)
    }

    fn living(game: &Game<'_>, id: u32) -> mod_api::LivingRef {
        game.native_living(game.native_entity(id).unwrap()).unwrap()
    }

    /// The ids of the modifiers a saved `attributes` list holds.
    fn saved_ids<'a>(list: impl IntoIterator<Item = &'a horizon_world::nbt::Tag>) -> Vec<String> {
        use horizon_client_core::nbt_value::text;
        use horizon_world::nbt::Tag;
        list.into_iter()
            .flat_map(|entry| match entry.get("modifiers") {
                Some(Tag::List(l)) => l.iter().filter_map(|m| text(m.get("id"))).collect(),
                _ => Vec::new(),
            })
            .collect()
    }

    use mod_api::{AttributeModifier, AttributeOperation, Identifier, ModifierLifetime};

    /// A mob's movement speed is kept where its gameplay reads it: a mod's
    /// permanent modifier is saved, a transient one is not, both count at once.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_mods_modifiers_on_a_mobs_speed_reach_its_gameplay() {
        use AttributeOperation::*;
        use ModifierLifetime::*;
        with_game(|game| {
            spawn(game);
            let speed = "minecraft:movement_speed";
            let zombie = living(game, 12);
            let (_, Some(base)) = read(game, 12, speed) else {
                panic!("a zombie has a movement speed");
            };
            assert!(zombie.add_attribute_modifier(
                game,
                speed,
                modifier("test:fast", 1.0, AddMultipliedBase),
                Permanent
            ));
            // The id is taken (vanilla throws).
            assert!(!zombie.add_attribute_modifier(
                game,
                speed,
                modifier("test:fast", 2.0, AddValue),
                Transient
            ));
            assert!(zombie.add_attribute_modifier(
                game,
                speed,
                modifier("test:boost", 0.5, AddMultipliedTotal),
                Transient
            ));
            let value = base * 2.0 * 1.5;
            assert_eq!(read(game, 12, speed), (Some(value), Some(base)));
            assert_eq!(game.mobs[0].attrs.speed, value);
            let saved = horizon_client::entity_nbt::attribute_entries(&game.mobs[0]);
            assert_eq!(saved_ids(saved.iter().map(|(_, e)| e)), ["test:fast"]);
            // Replacing takes the new lifetime: no longer saved.
            assert!(zombie.replace_attribute_modifier(
                game,
                speed,
                modifier("test:fast", 1.0, AddMultipliedBase),
                Transient
            ));
            let saved = horizon_client::entity_nbt::attribute_entries(&game.mobs[0]);
            assert!(saved_ids(saved.iter().map(|(_, e)| e)).is_empty());
            assert_eq!(read(game, 12, speed), (Some(value), Some(base)));
            let boost = Identifier::parse("test:boost").unwrap();
            assert!(zombie.remove_attribute_modifier(game, speed, &boost));
            assert!(!zombie.remove_attribute_modifier(game, speed, &boost));
            assert_eq!(read(game, 12, speed), (Some(base * 2.0), Some(base)));
            assert!(zombie.set_attribute_base(game, speed, 0.5));
            assert_eq!(read(game, 12, speed), (Some(1.0), Some(0.5)));
            assert_eq!(game.mobs[0].attrs.speed, 1.0);
        });
    }

    /// An attribute only the live map keeps keeps a mod's writes when the map
    /// is built again, and saves the permanent ones.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_mods_writes_outlive_a_rebuilt_map() {
        use AttributeOperation::*;
        use ModifierLifetime::*;
        with_game(|game| {
            spawn(game);
            let gravity = "minecraft:gravity";
            let zombie = living(game, 12);
            assert!(zombie.set_attribute_base(game, gravity, 0.1));
            assert!(zombie.add_attribute_modifier(
                game,
                gravity,
                modifier("test:heavy", 1.0, AddMultipliedBase),
                Permanent
            ));
            assert!(zombie.add_attribute_modifier(
                game,
                gravity,
                modifier("test:float", -0.05, AddValue),
                Transient
            ));
            let both = Some((0.1 - 0.05) * 2.0);
            assert_eq!(read(game, 12, gravity), (both, Some(0.1)));
            game.mobs[0].extra.live.invalidate();
            assert_eq!(read(game, 12, gravity), (both, Some(0.1)));
            let mut list = Vec::new();
            game.mobs[0].extra.live.save_into(&mut list);
            assert_eq!(saved_ids(&list), ["test:heavy"]);
            let float = Identifier::parse("test:float").unwrap();
            assert!(zombie.remove_attribute_modifier(game, gravity, &float));
            game.mobs[0].extra.live.invalidate();
            assert_eq!(read(game, 12, gravity), (Some(0.2), Some(0.1)));
            // A change to `finalize`'s store rebuilds the map without losing them.
            horizon_client::mobs::finalize::set_base(
                &mut game.mobs[0],
                horizon_client::mobs::finalize::Attr::MaxHealth,
                30.0,
            );
            assert_eq!(read(game, 12, gravity), (Some(0.2), Some(0.1)));
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_stand_the_dragon_and_a_player_take_writes() {
        use AttributeOperation::*;
        use ModifierLifetime::*;
        use horizon_client::motion::PlayerMotion;
        with_game(|game| {
            spawn(game);
            let health = "minecraft:max_health";
            let stand = living(game, 13);
            assert!(stand.set_attribute_base(game, health, 4.0));
            assert_eq!(read(game, 13, health), (Some(4.0), Some(4.0)));
            // Not an attribute of an armor stand; a non-finite amount.
            let damage = modifier("test:x", 1.0, AddValue);
            assert!(!stand.add_attribute_modifier(
                game,
                "minecraft:attack_damage",
                damage,
                Permanent
            ));
            assert!(!stand.add_attribute_modifier(
                game,
                health,
                modifier("test:x", f64::NAN, AddValue),
                Permanent
            ));
            let dragon = living(game, 14);
            assert!(dragon.add_attribute_modifier(
                game,
                health,
                modifier("test:tough", 100.0, AddValue),
                Transient
            ));
            assert_eq!(read(game, 14, health), (Some(300.0), Some(200.0)));

            game.player = Some(PlayerMotion::new(game.motion, Vec3::ZERO).unwrap());
            game.lan_acting_uuid = Some(123);
            game.lan_acting_entity = Some(20);
            let player = living(game, 20);
            assert!(player.add_attribute_modifier(
                game,
                health,
                modifier("test:hearty", 0.5, AddMultipliedBase),
                Permanent
            ));
            assert_eq!(read(game, 20, health), (Some(30.0), Some(20.0)));
            let hearty = Identifier::parse("test:hearty").unwrap();
            assert!(player.remove_attribute_modifier(game, health, &hearty));
            assert_eq!(read(game, 20, health), (Some(20.0), Some(20.0)));
            // A stale handle writes nothing.
            let zombie = living(game, 12);
            game.mobs.clear();
            assert!(!zombie.set_attribute_base(game, "minecraft:gravity", 1.0));
        });
    }
}
