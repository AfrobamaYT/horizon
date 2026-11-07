//! Vanilla 26.3 entity NBT (`Entity.saveWithoutId` and the subclasses'
//! `addAdditionalSaveData`) for the entities Horizon simulates: mobs, item
//! entities, experience orbs, arrows and falling blocks. Encoding starts from
//! the compound the entity was loaded with, when there is one, so fields
//! Horizon does not model (variants, attributes, equipment, custom names)
//! survive a Horizon save.
use crate::{
    arrows::Pickup,
    entities::{ArmorStand, Cushion, ExperienceOrb, ItemEntity, ItemFrame, Painting},
    mobs::{Mob, MobType, Who},
    nbt_path::int_value,
    projectiles::{Projectile, ProjectileKind},
    save::{
        doubles, float, floats, item_from_nbt, item_to_nbt, number, put, read_doubles, text,
        uuid_from_tag,
    },
};
use horizon_core::geometry::Vec3;
use std::{collections::HashMap, sync::Arc};
use {
    horizon_core::nbt::{Compound, Tag},
    horizon_world::{
        chunk::BlockState,
        registry::{BlockRegistry, BlockStateId},
    },
};

/// Saved data carried by `ConversionType.convertCommon` outside the modelled
/// mob fields. The converted entity has its own identity and type-specific data;
/// only its command tags are inherited from the old entity's raw compound.
pub fn conversion_tags(raw: Option<&Tag>) -> Option<Tag> {
    let Tag::List(tags) = raw?.get("Tags")? else {
        return None;
    };
    (!tags.is_empty()).then(|| Tag::compound([("Tags", Tag::List(tags.clone()))]))
}

fn string(s: &str) -> Tag {
    Tag::String(s.into())
}

fn hidden_effect_nbt(effect: &crate::effects::EffectInstance) -> Tag {
    let mut fields = vec![("duration", Tag::Int(effect.duration))];
    if effect.amplifier != 0 {
        fields.push((
            "amplifier",
            Tag::Byte(effect.amplifier.clamp(0, 255) as u8 as i8),
        ));
    }
    if effect.ambient {
        fields.push(("ambient", Tag::Byte(1)));
    }
    if !effect.visible {
        fields.push(("show_particles", Tag::Byte(0)));
    }
    fields.push(("show_icon", Tag::Byte(i8::from(effect.visible))));
    if let Some(hidden) = &effect.hidden {
        fields.push(("hidden_effect", hidden_effect_nbt(hidden)));
    }
    Tag::compound(fields)
}

fn hidden_effect_from_nbt(effect: &'static str, tag: &Tag) -> crate::effects::EffectInstance {
    let mut instance = crate::effects::EffectInstance::new(
        effect,
        number(tag.get("duration")).unwrap_or(0) as i32,
        number(tag.get("amplifier")).unwrap_or(0) as i32,
    );
    instance.ambient = number(tag.get("ambient")).is_some_and(|v| v != 0);
    instance.visible = number(tag.get("show_particles")) != Some(0);
    instance.hidden = tag
        .get("hidden_effect")
        .map(|hidden| Box::new(hidden_effect_from_nbt(effect, hidden)));
    instance
}
/// Entity type ID (`id`).
pub fn kind(tag: &Tag) -> Option<String> {
    horizon_client_core::identifier::normalize_id(&text(tag.get("id"))?)
}
pub fn position(tag: &Tag) -> Option<Vec3> {
    read_doubles::<3>(tag.get("Pos")).map(|p| Vec3::new(p[0], p[1], p[2]))
}
pub fn uuid(tag: &Tag) -> Option<[i32; 4]> {
    uuid_from_tag(tag.get("UUID"))
}
pub fn motion(tag: &Tag) -> Vec3 {
    read_doubles::<3>(tag.get("Motion")).map_or(Vec3::ZERO, |m| Vec3::new(m[0], m[1], m[2]))
}
pub fn rotation(tag: &Tag) -> [f32; 2] {
    read_doubles::<2>(tag.get("Rotation")).map_or([0.0; 2], |r| [r[0] as f32, r[1] as f32])
}
/// `NumericTag.byteValue`'s nonzero test for entity NBT, defaulting to false.
pub fn flag(tag: &Tag, key: &str) -> bool {
    // NBT floats floor before int narrowing; the final byte keeps the low bits.
    tag.get(key)
        .and_then(int_value)
        .is_some_and(|value| value as i8 != 0)
}
/// `Entity.saveWithoutId` fields every entity has, over the loaded compound.
struct Base<'a> {
    id: &'a str,
    uuid: [i32; 4],
    position: Vec3,
    motion: Vec3,
    rotation: [f32; 2],
    on_ground: bool,
    fall_distance: f64,
}
impl Base<'_> {
    fn tag(&self, raw: Option<&Tag>) -> Tag {
        let mut tag = match raw {
            Some(t @ Tag::Compound(_)) => t.clone(),
            _ => Tag::Compound(Compound::new()),
        };
        for (key, value) in [
            ("id", string(self.id)),
            ("UUID", Tag::IntArray(self.uuid.to_vec())),
            (
                "Pos",
                doubles(&[self.position.x, self.position.y, self.position.z]),
            ),
            (
                "Motion",
                doubles(&[self.motion.x, self.motion.y, self.motion.z]),
            ),
            ("Rotation", floats(&self.rotation)),
            ("OnGround", Tag::Byte(i8::from(self.on_ground))),
            ("fall_distance", Tag::Double(self.fall_distance.to_bits())),
        ] {
            put(&mut tag, key, value);
        }
        if let Tag::Compound(map) = &mut tag {
            for (key, value) in [
                // Entity.remainingFireTicks starts at 0 (-1 was the old default).
                ("Fire", Tag::Short(0)),
                ("Air", Tag::Short(300)),
                ("Invulnerable", Tag::Byte(0)),
                ("PortalCooldown", Tag::Int(0)),
            ] {
                map.entry(key.into()).or_insert(value);
            }
        }
        tag
    }
    /// Preserve Entity data without flags owned by Mob's save writer.
    /// Only the dedicated boat, item and armor-stand writers use this path.
    fn non_mob_tag(&self, raw: Option<&Tag>) -> Tag {
        let mut tag = self.tag(raw);
        if let Tag::Compound(map) = &mut tag {
            map.remove(&"NoAI".into());
            map.remove(&"PersistenceRequired".into());
        }
        tag
    }
}

/// `Entity.saveWithoutId`'s common fields for entities Horizon keeps as raw
/// compounds: each one is inserted only when the loaded compound has no value
/// for it, so explicit values survive.
pub fn fill_common_defaults(tag: &mut Tag) {
    let Tag::Compound(map) = tag else {
        return;
    };
    for (key, value) in [
        ("Motion", doubles(&[0.0, 0.0, 0.0])),
        ("OnGround", Tag::Byte(0)),
        ("fall_distance", Tag::Double(0.0f64.to_bits())),
        ("Air", Tag::Short(300)),
        ("Invulnerable", Tag::Byte(0)),
        ("PortalCooldown", Tag::Int(0)),
    ] {
        map.entry(key.into()).or_insert(value);
    }
}

/// The mobs that save `Age` and `AgeLocked`.
fn saves_age(s: crate::mobs::Species) -> bool {
    use crate::mobs::{Species, drops::can_be_baby, finalize::ageable};
    s == Species::Tadpole || (ageable(s) && can_be_baby(s))
}
/// `LivingEntity` + `Mob` (+ `AgeableMob`, creeper and skeleton specifics).
/// The mobs whose class extends vanilla's `Animal` (and so save `InLove`):
/// not allays, villagers, golems, dolphins or fish.
fn is_animal(id: &str) -> bool {
    matches!(
        id.strip_prefix("minecraft:").unwrap_or(id),
        "pig"
            | "cow"
            | "mooshroom"
            | "sheep"
            | "chicken"
            | "rabbit"
            | "horse"
            | "donkey"
            | "mule"
            | "zombie_horse"
            | "skeleton_horse"
            | "llama"
            | "trader_llama"
            | "goat"
            | "panda"
            | "polar_bear"
            | "fox"
            | "wolf"
            | "cat"
            | "ocelot"
            | "parrot"
            | "bee"
            | "turtle"
            | "armadillo"
            | "frog"
            | "axolotl"
            | "hoglin"
            | "strider"
            | "camel"
            | "camel_husk"
            | "sniffer"
            | "happy_ghast"
            | "nautilus"
            | "zombie_nautilus"
    )
}
/// The `NeutralMob`s whose anger is `Extra::anger`: they write and read
/// `anger_end_time` and `angry_at`. The bee keeps its own anger, and a
/// piglin's is a brain memory, not `NeutralMob`'s.
fn persistent_anger(s: crate::mobs::Species) -> bool {
    use crate::mobs::Species as S;
    matches!(
        s,
        S::Wolf | S::IronGolem | S::PolarBear | S::Enderman | S::ZombifiedPiglin
    )
}
/// The single player's UUID (`offline_uuid("Player")`): the entity the
/// neutral mobs are angry at.
fn player_uuid() -> [i32; 4] {
    let u = horizon_protocol::crypto::offline_uuid("Player");
    [
        (u >> 96) as u32 as i32,
        (u >> 64) as u32 as i32,
        (u >> 32) as u32 as i32,
        u as u32 as i32,
    ]
}
/// The UUID a save writes for a reference to `who`. A mob is only an id here:
/// its UUID lives in the game layer (`EntityMemory.uuids`), which is what
/// `uuid_of` answers from. When it answers `None` (the mob is no longer in the
/// level) nothing is written, rather than a made-up UUID that would name nothing
/// when the save is read back. The reference comes back with [`who_by_uuid`].
fn who_uuid(who: Who, uuid_of: &mut impl FnMut(u32) -> Option<[i32; 4]>) -> Option<[i32; 4]> {
    match who {
        Who::Player => Some(player_uuid()),
        Who::Mob(id) => uuid_of(id),
    }
}
/// The entity a stored UUID names, while it is in the level: `player` (the
/// player's own UUID, when one is in the level), or the mob `uuids` gives that
/// UUID to. `uuids` is the game layer's id -> UUID map, so this is the read half
/// of the pair [`who_uuid`] and `mob_to_nbt`'s `uuid_of` form. A UUID nothing
/// loaded has, or one naming a kind `Who` cannot carry (an item, a projectile, a
/// guest), is `None` - never some other entity as a stand-in.
pub fn who_by_uuid(
    uuids: &HashMap<u32, [i32; 4]>,
    mobs: &[Mob],
    player: Option<[i32; 4]>,
    uuid: [i32; 4],
) -> Option<Who> {
    if player == Some(uuid) {
        return Some(Who::Player);
    }
    // Only a loaded mob: the map also holds projectiles, vehicles and items,
    // whose ids mean nothing to `Who`.
    uuids
        .iter()
        .find(|(id, u)| **u == uuid && mobs.iter().any(|m| m.id == **id))
        .map(|(id, _)| Who::Mob(*id))
}
/// `ResolvableProfile.CODEC`: the name, id and properties of a profile, then the
/// skin's texture ids and model (`PlayerSkin.Patch`: "texture", "cape", "elytra", "model").
fn profile_nbt(p: &horizon_protocol::entity_data::Profile) -> Tag {
    let mut entries: Vec<(&'static str, Tag)> = Vec::new();
    if let Some(name) = &p.name {
        entries.push(("name", string(name)));
    }
    if let Some(id) = p.uuid {
        entries.push((
            "id",
            Tag::IntArray(vec![
                (id >> 96) as i32,
                (id >> 64) as i32,
                (id >> 32) as i32,
                id as i32,
            ]),
        ));
    }
    if !p.properties.is_empty() {
        let properties = p
            .properties
            .iter()
            .map(|property| {
                let mut entries: Vec<(&'static str, Tag)> = vec![
                    ("name", string(&property.name)),
                    ("value", string(&property.value)),
                ];
                if let Some(signature) = &property.signature {
                    entries.push(("signature", string(signature)));
                }
                Tag::compound(entries)
            })
            .collect();
        entries.push(("properties", Tag::List(properties)));
    }
    for (key, texture) in [
        ("texture", &p.body),
        ("cape", &p.cape),
        ("elytra", &p.elytra),
    ] {
        if let Some(texture) = texture {
            entries.push((key, string(texture)));
        }
    }
    if let Some(slim) = p.slim {
        entries.push(("model", string(if slim { "slim" } else { "wide" })));
    }
    Tag::compound(entries)
}
/// `ResolvableProfile.CODEC` read back: the compound `profile_nbt` writes, or a
/// bare player name (`Codec.withAlternative(FULL_CODEC, PLAYER_NAME, ...)`).
fn profile_from_nbt(tag: &Tag) -> Option<horizon_protocol::entity_data::Profile> {
    if let Tag::String(name) = tag {
        return Some(horizon_protocol::entity_data::Profile {
            name: Some(name.to_string_lossy()),
            uuid: None,
            properties: Vec::new(),
            body: None,
            cape: None,
            elytra: None,
            slim: None,
        });
    }
    let property = |name: String, value: &Tag, signature: Option<String>| match value {
        Tag::String(v) => Some(horizon_protocol::packets::login::Property {
            name,
            value: v.to_string_lossy(),
            signature,
        }),
        _ => None,
    };
    let properties: Vec<horizon_protocol::packets::login::Property> = match tag.get("properties") {
        // `ExtraCodecs.PROPERTY_MAP`: a list of properties ...
        Some(Tag::List(list)) => list
            .iter()
            .filter_map(|p| {
                let name = match p.get("name")? {
                    Tag::String(s) => s.to_string_lossy(),
                    _ => return None,
                };
                let signature = match p.get("signature") {
                    Some(Tag::String(s)) => Some(s.to_string_lossy()),
                    _ => None,
                };
                property(name, p.get("value")?, signature)
            })
            .collect(),
        // ... or a map from a name to its values.
        Some(Tag::Compound(map)) => map
            .iter()
            .flat_map(|(name, values)| match values {
                Tag::List(values) => values
                    .iter()
                    .filter_map(|v| property(name.to_string_lossy(), v, None))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let text = |key: &str| match tag.get(key) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    Some(horizon_protocol::entity_data::Profile {
        name: text("name"),
        uuid: uuid_from_tag(tag.get("id")).map(|a| {
            a.iter()
                .fold(0u128, |u, &x| (u << 32) | u128::from(x as u32))
        }),
        properties,
        body: text("texture"),
        cape: text("cape"),
        elytra: text("elytra"),
        slim: text("model").map(|model| model == "slim"),
    })
}
/// `ConversionTracker.addAdditionalSaveData`: vanilla's conversion time, -1
/// while the mob is not converting. `counter2` is that time plus one while it
/// converts and 0 when it does not (see `mobs/undead.rs`).
fn conversion_time(m: &Mob) -> i32 {
    if m.extra.counter2 > 0 {
        m.extra.counter2 - 1
    } else {
        -1
    }
}
/// `BlockState.CODEC`: the block's name when the state is that block's
/// default, `{id, properties}` for any other state.
pub fn state_to_nbt(registry: &BlockRegistry, state: BlockStateId) -> Tag {
    match registry.state(state) {
        Some(s) if registry.default_state(&s.name) == Some(state) => string(&s.name),
        Some(s) => s.to_nbt(),
        None => string("minecraft:air"),
    }
}
/// `BlockState.CODEC` read back: a block name or `{id, properties}` (or the
/// legacy `{Name, Properties}`). A name the registry knows only as a block
/// reads as its default state, and one it does not know at all as `None`.
pub fn state_from_nbt(registry: &BlockRegistry, tag: &Tag) -> Option<BlockStateId> {
    let state = match tag.get("Name") {
        Some(Tag::String(name)) => BlockState {
            name: name.to_string_lossy(),
            properties: match tag.get("Properties") {
                Some(Tag::Compound(p)) => p
                    .iter()
                    .filter_map(|(k, v)| match v {
                        Tag::String(v) => Some((k.to_string_lossy(), v.to_string_lossy())),
                        _ => None,
                    })
                    .collect(),
                _ => Default::default(),
            },
        },
        _ => BlockState::from_nbt(tag).ok()?,
    };
    registry
        .id(&state)
        .or_else(|| registry.default_state(&state.name))
}
/// `BlockState.isAir`: air, cave air and void air.
fn is_air(registry: &BlockRegistry, state: BlockStateId) -> bool {
    registry.state(state).is_some_and(|s| {
        matches!(
            s.name.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}
/// The mob's entity compound. `registry` is what writes a field holding a block
/// state (an enderman's `carriedBlockState`): without one, a caller that has no
/// resources (unit tests, structure templates) keeps whatever the loaded
/// compound had for it.
///
/// `uuid_of` is how a field that names another entity by UUID (a mob's
/// `angry_at`) is written: it answers the UUID of a mob by its id, from the game
/// layer's map, and `None` for one the level no longer has. A caller with no
/// such map passes `|_| None` and writes no reference.
pub fn mob_to_nbt(
    m: &Mob,
    uuid: [i32; 4],
    raw: Option<&Tag>,
    registry: Option<&BlockRegistry>,
    mut uuid_of: impl FnMut(u32) -> Option<[i32; 4]>,
) -> Tag {
    use crate::mobs::Species as S;
    let base = Base {
        id: &m.kind.id,
        uuid,
        position: m.position,
        motion: m.velocity,
        rotation: [m.yaw, m.head_pitch],
        on_ground: m.on_ground,
        fall_distance: m.fall_distance,
    };
    let mut tag = base.tag(raw);
    for (key, value) in [
        (
            "Fire",
            Tag::Short(m.fire_ticks.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16),
        ),
        ("Air", Tag::Short(m.air as i16)),
        ("Health", Tag::Float(m.health.max(0.0).to_bits())),
        ("AbsorptionAmount", Tag::Float(m.absorption.to_bits())),
        (
            "HurtTime",
            Tag::Short(m.hurt_time.clamp(0, i32::from(i16::MAX)) as i16),
        ),
        (
            "DeathTime",
            Tag::Short(m.death_time.clamp(0, i32::from(i16::MAX)) as i16),
        ),
        // Mob.addAdditionalSaveData writes the current virtual getter, not raw input.
        ("CanPickUpLoot", Tag::Byte(i8::from(m.can_pick_up_loot()))),
        ("PersistenceRequired", Tag::Byte(i8::from(m.persistent))),
    ] {
        put(&mut tag, key, value);
    }
    // Mob.addAdditionalSaveData: hasHome means radius != -1, including -2.
    // Species writers below retain their own home_pos override.
    let Tag::Compound(map) = &mut tag else {
        unreachable!()
    };
    if let Some((pos, radius)) = m.ai.home.filter(|(_, radius)| *radius != -1) {
        map.insert("home_radius".into(), Tag::Int(radius));
        map.insert("home_pos".into(), Tag::IntArray(pos.to_vec()));
    } else {
        map.remove(&"home_radius".into());
        map.remove(&"home_pos".into());
    }
    // AbstractFish/Axolotl.addAdditionalSaveData always write the flag.
    // Tadpole.fromBucket is always true, and its setter ignores loaded input.
    let from_bucket = match m.species() {
        S::Tadpole => Some(true),
        S::Cod | S::Salmon | S::Pufferfish | S::TropicalFish | S::Axolotl => {
            Some(m.extra.from_bucket)
        }
        _ => None,
    };
    if let Some(value) = from_bucket {
        put(&mut tag, "FromBucket", Tag::Byte(i8::from(value)));
    }
    // Mob.addAdditionalSaveData: the death loot table, and its seed when set.
    if let Some(table) = &m.extra.death_loot_table {
        put(&mut tag, "DeathLootTable", string(table));
    }
    if m.extra.death_loot_seed != 0 {
        put(
            &mut tag,
            "DeathLootTableSeed",
            Tag::Long(m.extra.death_loot_seed),
        );
    }
    // `NeutralMob.addPersistentAngerSaveData`: the anger ends at an
    // absolute game time, and the target is the entity the mob is angry at.
    if persistent_anger(m.species()) {
        put(
            &mut tag,
            "anger_end_time",
            Tag::Long(if m.extra.anger_end > 0 {
                m.extra.anger_end
            } else {
                -1
            }),
        );
        // `angry_at` only while there is an anger target: the UUID of the one
        // the level has (`uuid_of`). A mob target read from the save that is
        // not in the world (`Who::Mob(0)`) keeps the loaded UUID.
        match m.extra.angry_at {
            Some(crate::mobs::Who::Mob(0)) => {}
            target => match target.and_then(|who| who_uuid(who, &mut uuid_of)) {
                Some(uuid) => put(&mut tag, "angry_at", Tag::IntArray(uuid.to_vec())),
                None => {
                    if let Tag::Compound(map) = &mut tag {
                        map.remove(&"angry_at".into());
                    }
                }
            },
        }
    }
    // `Mannequin.addAdditionalSaveData`: the profile whose skin it wears, whether it
    // is `immovable`, and its description (the default is not written; none is `hide_description`).
    if m.species() == crate::mobs::Species::Mannequin {
        if let Some(profile) = &m.profile {
            put(&mut tag, "profile", profile_nbt(profile));
        }
        put(
            &mut tag,
            "immovable",
            Tag::Byte(i8::from(m.extra.immovable)),
        );
        match &m.extra.description {
            crate::mobs::MannequinDescription::Default => {}
            crate::mobs::MannequinDescription::Hidden => {
                put(&mut tag, "hide_description", Tag::Byte(1));
            }
            crate::mobs::MannequinDescription::Custom(component) => {
                put(&mut tag, "description", component.clone());
            }
        }
    }
    // `Entity.saveWithoutId` writes the flag only when set.
    if m.name_visible {
        put(&mut tag, "CustomNameVisible", Tag::Byte(1));
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"CustomNameVisible".into());
    }
    // A styled name from the save stays unless the name changed.
    if custom_name(&tag) != m.custom_name
        && let Some(name) = &m.custom_name
    {
        put(&mut tag, "CustomName", Tag::String(name.as_str().into()));
    }
    let Tag::Compound(map) = &mut tag else {
        return tag;
    };
    for (key, on) in [
        ("NoAI", m.no_ai),
        ("Silent", m.silent),
        ("NoGravity", m.no_gravity),
        ("Glowing", m.glowing),
    ] {
        if on {
            map.insert(key.into(), Tag::Byte(1));
        } else {
            map.remove(&horizon_core::nbt::JavaString::from(key));
        }
    }
    // LivingEntity.addAdditionalSaveData: `active_effects` only when there are
    // any (MobEffectInstance.CODEC; a mob's effects are never ambient, and one
    // given with `hideParticles` shows neither particles nor its icon).
    if m.extra.effects.is_empty() {
        map.remove(&horizon_core::nbt::JavaString::from("active_effects"));
    } else {
        let effects = m.extra.effects.iter().map(|effect| {
            let (effect, duration, amplifier) = (effect.id, effect.duration, effect.amplifier);
            let shown = !m.extra.hidden_effects.contains(&effect);
            let ambient = m.extra.ambient_effects.contains(&effect);
            let mut e = vec![("id", string(crate::brewing::namespaced(effect)))];
            // The codec leaves out an amplifier of 0 (its default).
            if amplifier != 0 {
                e.push(("amplifier", Tag::Byte(amplifier.clamp(0, 255) as u8 as i8)));
            }
            e.push(("duration", Tag::Int(duration)));
            if ambient {
                e.push(("ambient", Tag::Byte(1)));
            }
            if !shown {
                e.push(("show_particles", Tag::Byte(0)));
            }
            e.push(("show_icon", Tag::Byte(i8::from(shown))));
            if let Some(hidden) = m.extra.effect_chains.iter().find(|e| e.id == effect) {
                e.push(("hidden_effect", hidden_effect_nbt(hidden)));
            }
            Tag::compound(e)
        });
        map.insert("active_effects".into(), Tag::List(effects.collect()));
    }
    map.insert(
        "Invulnerable".into(),
        Tag::Byte(i8::from(m.invulnerable_flag)),
    );
    map.insert(
        "LeftHanded".into(),
        Tag::Byte(i8::from(m.extra.left_handed)),
    );
    // Keys of older formats that structure templates still carry.
    for key in [
        "HandItems",
        "ArmorItems",
        "HandDropChances",
        "ArmorDropChances",
        "Dimension",
        "HurtByTimestamp",
        "Leashed",
    ] {
        map.remove(&horizon_core::nbt::JavaString::from(key));
    }
    if m.ticks_frozen > 0 {
        map.insert("TicksFrozen".into(), Tag::Int(m.ticks_frozen));
    } else {
        map.remove(&horizon_core::nbt::JavaString::from("TicksFrozen"));
    }
    for (key, value) in [
        ("AbsorptionAmount", Tag::Float(0)),
        ("FallFlying", Tag::Byte(0)),
        ("current_impulse_context_reset_grace_time", Tag::Int(0)),
        (
            "Brain",
            Tag::compound([("memories", Tag::Compound(Compound::new()))]),
        ),
    ] {
        map.entry(key.into()).or_insert(value);
    }
    // What the mob's class saves: a content mod's living type saves as its
    // template does (the `id` written above stays its own).
    let id = m.kind.class_id();
    // AgeableMob.addAdditionalSaveData, only if canBeABaby (not a frog or a
    // parrot); a tadpole saves its own age.
    let species = m.species();
    if saves_age(species) {
        map.insert("Age".into(), Tag::Int(m.extra.age));
        map.insert("AgeLocked".into(), Tag::Byte(i8::from(m.extra.age_locked)));
        if species != crate::mobs::Species::Tadpole {
            map.insert("ForcedAge".into(), Tag::Int(m.extra.forced_age));
        }
    } else if matches!(
        species,
        crate::mobs::Species::Parrot
            | crate::mobs::Species::Slime
            | crate::mobs::Species::MagmaCube
            | crate::mobs::Species::ZombieNautilus
    ) {
        // AgeableMob.addAdditionalSaveData writes nothing when canBeABaby is
        // false: drop the ignored values a raw input compound carried.
        for key in ["Age", "ForcedAge", "AgeLocked"] {
            map.remove(&key.into());
        }
    }
    // Animal.addAdditionalSaveData: the love ticks left, whatever the save had.
    if is_animal(id) {
        map.insert("InLove".into(), Tag::Int(m.extra.in_love));
        // Animal.addAdditionalSaveData: the player who fed it.
        if let Some(cause) = m.extra.love_cause {
            map.insert("LoveCause".into(), Tag::IntArray(cause.to_vec()));
        } else {
            map.remove(&"LoveCause".into());
        }
    }
    // InventoryCarrier.writeInventoryToTag: the occupied stacks, even when empty.
    if matches!(id, "minecraft:pillager" | "minecraft:wandering_trader") {
        map.insert(
            "Inventory".into(),
            Tag::List(
                m.extra
                    .inventory
                    .iter()
                    .map(crate::save::item_to_nbt)
                    .collect(),
            ),
        );
    }
    let species = m.species();
    // `VariantUtils.writeVariant`: only the species that store the name. A
    // mooshroom saves `Type` (below), a fox `Type`, a horse and a llama a
    // packed `Variant` int, so those report the component but save no name.
    if crate::mobs::finalize::saves_variant_name(species)
        && let Some((_, variant)) =
            crate::mobs::finalize::species_variant_component(species, m.extra.variant)
    {
        map.insert("variant".into(), string(&variant));
    }
    if species == crate::mobs::Species::Chicken {
        map.insert("EggLayTime".into(), Tag::Int(m.extra.egg_time));
    }
    let sounds = crate::mobs::finalize::sound_variants(species);
    if !sounds.is_empty() {
        let sound = m.extra.sound_variant.and_then(|i| sounds.get(i as usize));
        let sound = sound.copied().unwrap_or("classic");
        map.insert(
            "sound_variant".into(),
            string(&format!("minecraft:{sound}")),
        );
    }
    match id {
        // Fox.addAdditionalSaveData; Horizon models only the type, the
        // flags and no trusted players yet.
        "minecraft:fox" => {
            let kind = if m.extra.variant == 1 { "snow" } else { "red" };
            map.insert("Type".into(), string(kind));
            // The trusted UUIDs (`Fox.TRUSTED_LIST_CODEC`: a list of int arrays).
            map.insert(
                "Trusted".into(),
                Tag::List(
                    crate::mobs::fox::trusted_entities(m)
                        .map(|uuid| Tag::IntArray(uuid.to_vec()))
                        .collect(),
                ),
            );
            for (key, bit) in [
                ("Sleeping", crate::mobs::fox::SLEEPING),
                ("Sitting", crate::mobs::fox::SITTING),
                ("Crouching", crate::mobs::fox::CROUCHING),
            ] {
                map.insert(
                    key.into(),
                    Tag::Byte(i8::from(crate::mobs::fox::flag(m, bit))),
                );
            }
        }
        // MushroomCow.addAdditionalSaveData: the Variant codec's name and the
        // stew effects, written only while it holds any (storeNullable).
        "minecraft:mooshroom" => {
            let kind = if m.extra.variant == 1 { "brown" } else { "red" };
            map.insert("Type".into(), string(kind));
            match &m.extra.stew_effects {
                Some(effects) => {
                    let list = effects.iter().map(|(id, duration)| {
                        let mut entry = vec![("id", string(id))];
                        // SuspiciousStewEffects.Entry leaves out the default.
                        if *duration != 160 {
                            entry.push(("duration", Tag::Int(*duration)));
                        }
                        Tag::compound(entry)
                    });
                    map.insert("stew_effects".into(), Tag::List(list.collect()));
                }
                None => {
                    map.remove(&horizon_core::nbt::JavaString::from("stew_effects"));
                }
            }
        }
        // Rabbit.addAdditionalSaveData (the legacy id; 99 is the killer bunny).
        "minecraft:rabbit" => {
            map.insert("RabbitType".into(), Tag::Int(i32::from(m.extra.variant)));
            map.insert(
                "MoreCarrotTicks".into(),
                Tag::Int(m.extra.more_carrot_ticks),
            );
        }
        "minecraft:goat" => {
            map.insert(
                "IsScreamingGoat".into(),
                Tag::Byte(i8::from(m.extra.screaming)),
            );
            let [left, right] = m.extra.lost_horns;
            map.insert("HasLeftHorn".into(), Tag::Byte(i8::from(!left)));
            map.insert("HasRightHorn".into(), Tag::Byte(i8::from(!right)));
        }
        "minecraft:creeper" => {
            map.insert("Fuse".into(), Tag::Short(m.max_swell as i16));
            map.insert(
                "ExplosionRadius".into(),
                Tag::Byte(m.extra.explosion_radius),
            );
            map.insert("powered".into(), Tag::Byte(i8::from(m.extra.charged)));
            map.insert("ignited".into(), Tag::Byte(i8::from(m.extra.flag)));
        }
        // `Ocelot.addAdditionalSaveData`.
        "minecraft:ocelot" => {
            map.insert("Trusting".into(), Tag::Byte(i8::from(m.extra.trusting)));
        }
        "minecraft:cat" => {
            if let Some(owner) = m.extra.owner {
                map.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
            } else {
                map.remove(&"Owner".into());
            }
            map.insert("Sitting".into(), Tag::Byte(i8::from(m.extra.sitting)));
            map.insert("CollarColor".into(), Tag::Byte(m.extra.color as i8));
        }
        "minecraft:wolf" => {
            if let Some(owner) = m.extra.owner {
                map.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
            } else {
                map.remove(&"Owner".into());
            }
            map.insert("Sitting".into(), Tag::Byte(i8::from(m.extra.sitting)));
            map.insert("CollarColor".into(), Tag::Byte(m.extra.color as i8));
        }
        "minecraft:panda" => {
            use crate::mobs::panda::GENES;
            for (key, gene) in [
                ("MainGene", m.extra.variant),
                ("HiddenGene", m.extra.variant2),
            ] {
                map.insert(
                    key.into(),
                    string(GENES[usize::from(gene).min(GENES.len() - 1)]),
                );
            }
        }
        "minecraft:piglin" | "minecraft:piglin_brute" | "minecraft:hoglin" => {
            // Only Piglin.addAdditionalSaveData writes CannotHunt.
            if id == "minecraft:piglin_brute" {
                map.remove(&"CannotHunt".into());
            }
            map.insert(
                "IsImmuneToZombification".into(),
                Tag::Byte(i8::from(m.extra.flag2)),
            );
            map.insert("TimeInOverworld".into(), Tag::Int(m.extra.counter2));
            if id == "minecraft:hoglin" {
                map.insert(
                    "CannotBeHunted".into(),
                    Tag::Byte(m.extra.cannot_be_hunted as i8),
                );
            }
            if id == "minecraft:piglin" {
                write_guarded_piglin_anger(m, map);
                // InventoryCarrier.writeInventoryToTag.
                map.insert(
                    "Inventory".into(),
                    Tag::List(
                        m.extra
                            .inventory
                            .iter()
                            .map(crate::save::item_to_nbt)
                            .collect(),
                    ),
                );
            }
        }
        "minecraft:horse"
        | "minecraft:donkey"
        | "minecraft:mule"
        | "minecraft:skeleton_horse"
        | "minecraft:zombie_horse"
        | "minecraft:llama"
        | "minecraft:trader_llama" => {
            map.insert("Tame".into(), Tag::Byte(i8::from(m.extra.tame)));
            map.insert("Temper".into(), Tag::Int(i32::from(m.extra.variant3)));
            map.insert("Bred".into(), Tag::Byte(i8::from(m.extra.bred)));
            // AbstractHorse.addAdditionalSaveData; Horizon does not model the
            // eating haystack yet.
            map.entry("EatingHaystack".into()).or_insert(Tag::Byte(0));
            if let Some(owner) = m.extra.owner {
                map.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
            }
            if id == "minecraft:horse" {
                let v = i32::from(m.extra.variant) | i32::from(m.extra.variant2) << 8;
                map.insert("Variant".into(), Tag::Int(v));
            }
            let llama = crate::mobs::llama::is_llama(m.species());
            if llama {
                crate::mobs::llama::to_nbt(m, map);
            }
            // Only llamas, donkeys and mules carry chests.
            if llama || matches!(id, "minecraft:donkey" | "minecraft:mule") {
                map.insert("ChestedHorse".into(), Tag::Byte(i8::from(m.extra.chested)));
            }
            if m.extra.chested && !m.extra.items.is_empty() {
                map.insert("Items".into(), crate::save::encode_items(&m.extra.items));
            } else {
                map.remove(&"Items".into());
            }
            // The `equipment` compound is written below, for every mob.
            let modelled = [
                ("minecraft:max_health", f64::from(m.attrs.max_health)),
                ("minecraft:movement_speed", m.attrs.speed),
                ("minecraft:jump_strength", m.extra.vector[0]),
            ];
            // AttributeMap.pack writes the instances that exist: the ones the
            // entity was loaded with, or all three randomized by finalizeSpawn.
            let attributes = match map.get(&"attributes".into()) {
                Some(Tag::List(list)) => list
                    .iter()
                    .map(|a| {
                        let id = text(a.get("id"));
                        let value = modelled.iter().find(|(m, _)| Some(*m) == id.as_deref());
                        match (a, value) {
                            (Tag::Compound(c), Some((_, v))) => {
                                let mut c = c.clone();
                                c.insert("base".into(), Tag::Double(v.to_bits()));
                                Tag::Compound(c)
                            }
                            _ => a.clone(),
                        }
                    })
                    .collect(),
                _ => modelled
                    .iter()
                    .map(|(id, v)| {
                        Tag::compound([("id", string(id)), ("base", Tag::Double(v.to_bits()))])
                    })
                    .collect(),
            };
            map.insert("attributes".into(), Tag::List(attributes));
        }
        "minecraft:bat" => {
            map.insert("BatFlags".into(), Tag::Byte(i8::from(m.extra.flag)));
        }
        "minecraft:turtle" => {
            map.insert("has_egg".into(), Tag::Byte(i8::from(m.extra.flag)));
            if let Some(home) = m.extra.home {
                map.insert("home_pos".into(), Tag::IntArray(home.to_vec()));
            }
        }
        "minecraft:creaking" => {
            // home_pos: the heart it is bound to.
            if let Some(home) = m.extra.home {
                map.insert("home_pos".into(), Tag::IntArray(home.to_vec()));
            } else {
                map.remove(&"home_pos".into());
            }
        }
        "minecraft:zombie_villager" => {
            map.insert("VillagerData".into(), villager_data(m));
            map.insert("VillagerDataFinalized".into(), Tag::Byte(1));
            let xp = m.extra.trade.as_deref().map_or(0, |t| t.xp);
            map.insert("Xp".into(), Tag::Int(xp));
            put_offers(map, m);
            crate::mobs::zombie_villager::write_nbt(m, map);
        }
        "minecraft:villager" => {
            map.insert("VillagerData".into(), villager_data(m));
            // A loaded villager with VillagerData is finalized.
            map.insert("VillagerDataFinalized".into(), Tag::Byte(1));
            let t = m.extra.trade.as_deref().cloned().unwrap_or_default();
            map.insert("Xp".into(), Tag::Int(t.xp));
            map.insert("LastRestock".into(), Tag::Long(t.last_restock));
            map.insert("RestocksToday".into(), Tag::Int(t.restocks_today));
            put_offers(map, m);
            put_memories(map, m);
            crate::mobs::villager::write_nbt(m, map);
        }
        // Dolphin.addAdditionalSaveData: the fish it was fed, and how wet it is.
        "minecraft:dolphin" => {
            map.insert("GotFish".into(), Tag::Byte(i8::from(m.extra.flag)));
            map.insert("Moistness".into(), Tag::Int(m.extra.counter));
        }
        "minecraft:wandering_trader" => {
            put_offers(map, m);
            // WanderingTrader.addAdditionalSaveData: the visit's countdown and
            // the place it is walking to, a BlockPos codec (an int array).
            map.insert("DespawnDelay".into(), Tag::Int(m.extra.ints[5]));
            if let Some(p) = m.extra.point {
                // The trader keeps its target as a position; the tag is a
                // BlockPos, so only the block it is in is saved.
                map.insert(
                    "wander_target".into(),
                    Tag::IntArray(vec![
                        p[0].floor() as i32,
                        p[1].floor() as i32,
                        p[2].floor() as i32,
                    ]),
                );
            }
        }
        "minecraft:axolotl" => {
            write_axolotl_memories(m, map);
            map.insert(
                "Variant".into(),
                Tag::Int(i32::from(m.extra.variant.min(4))),
            );
        }
        "minecraft:parrot" => {
            map.insert(
                "Variant".into(),
                Tag::Int(i32::from(m.extra.variant.min(4))),
            );
            if let Some(owner) = m.extra.owner {
                map.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
            } else {
                map.remove(&"Owner".into());
            }
            map.insert("Sitting".into(), Tag::Byte(i8::from(m.extra.sitting)));
        }
        "minecraft:salmon" => {
            let size = SALMON_SIZES[usize::from(m.extra.variant).min(2)];
            map.insert("type".into(), string(size));
        }
        // Pufferfish.addAdditionalSaveData: the puff state (0 small, 1 mid,
        // 2 full).
        "minecraft:pufferfish" => {
            map.insert("PuffState".into(), Tag::Int(m.extra.size));
        }
        // TropicalFish.addAdditionalSaveData: the packed variant.
        "minecraft:tropical_fish" => {
            let pattern = m.extra.variant.min(11);
            let packed = i32::from(pattern >= 6)
                | i32::from(pattern % 6) << 8
                | i32::from(m.extra.variant2 & 15) << 16
                | i32::from(m.extra.variant3 & 15) << 24;
            map.insert("Variant".into(), Tag::Int(packed));
        }
        "minecraft:allay" => {
            // InventoryCarrier: the fetched stack; the liked player memory.
            let inv = crate::mobs::allay::inventory(m)
                .map(item_to_nbt)
                .into_iter()
                .collect();
            map.insert("Inventory".into(), Tag::List(inv));
            if let Some(st) = &m.extra.allay {
                map.insert("listener".into(), st.vibration.to_nbt());
            }
            map.insert(
                "DuplicationCooldown".into(),
                Tag::Long(i64::from(m.extra.egg_time)),
            );
            let mut memories: Vec<(&str, Tag)> = Vec::new();
            if m.extra.tame {
                memories.push((
                    "minecraft:liked_player",
                    Tag::compound([(
                        "value",
                        Tag::IntArray(m.extra.owner.unwrap_or([0, 0, 0, 1]).to_vec()),
                    )]),
                ));
            }
            // GlobalPos.CODEC, and the cooldown while it runs.
            if let Some(liked) = m.extra.allay.as_ref().and_then(|st| st.noteblock) {
                memories.push((
                    "minecraft:liked_noteblock",
                    Tag::compound([(
                        "value",
                        Tag::compound([
                            ("dimension", string(liked.dimension)),
                            ("pos", Tag::IntArray(liked.pos.to_vec())),
                        ]),
                    )]),
                ));
            }
            if m.extra.ints[2] > 0 {
                memories.push((
                    "minecraft:liked_noteblock_cooldown_ticks",
                    Tag::compound([("value", Tag::Int(m.extra.ints[2]))]),
                ));
            }
            // Always written, so a forgotten note block does not come back
            // from the compound the allay was loaded with.
            map.insert(
                "Brain".into(),
                Tag::compound([("memories", Tag::compound(memories))]),
            );
        }
        "minecraft:camel" | "minecraft:camel_husk" => {
            // LastPoseTick: the pose change time, negative while sitting. The
            // game time is not known here; the world's save overwrites it
            // with the real one (`camel::synced_pose_tick`).
            map.insert(
                "LastPoseTick".into(),
                Tag::Long(crate::mobs::camel::synced_pose_tick(m, 0)),
            );
        }
        "minecraft:nautilus" | "minecraft:zombie_nautilus" => {
            crate::mobs::nautilus::to_nbt(m, map);
        }
        "minecraft:sulfur_cube" => crate::mobs::sulfur_cube::to_nbt(m, map),
        "minecraft:bee" => crate::mobs::bee::to_nbt(m, map),
        "minecraft:sniffer" => crate::mobs::sniffer::to_nbt(m, map),
        "minecraft:wither" => {
            map.insert("Invul".into(), Tag::Int(m.extra.counter));
        }
        // IronGolem.addAdditionalSaveData.
        "minecraft:iron_golem" => {
            map.insert(
                "PlayerCreated".into(),
                Tag::Byte(i8::from(m.extra.player_created)),
            );
        }
        "minecraft:warden" => {
            if let Some(st) = &m.extra.warden {
                map.insert("listener".into(), st.vibration.to_nbt());
                map.insert("anger".into(), Tag::compound(Vec::<(&str, Tag)>::new()));
            }
        }
        // Armadillo.addAdditionalSaveData: the time left to the next scute.
        "minecraft:armadillo" => {
            map.insert("scute_time".into(), Tag::Int(m.extra.egg_time));
        }
        "minecraft:ravager" => {
            map.insert("AttackTick".into(), Tag::Int(m.extra.counter));
            map.insert("StunTick".into(), Tag::Int(m.extra.counter2));
            map.insert("RoarTick".into(), Tag::Int(m.extra.anim_ticks));
        }
        "minecraft:vex" => {
            if let Some(h) = m.extra.home {
                map.insert("bound_pos".into(), Tag::IntArray(h.to_vec()));
            }
            if m.extra.egg_time > 0 {
                map.insert("life_ticks".into(), Tag::Int(m.extra.egg_time));
            }
        }
        "minecraft:endermite" => {
            map.insert("Lifetime".into(), Tag::Int(m.extra.counter));
        }
        // Enderman.addAdditionalSaveData: the block it carries, saved only
        // while it carries one.
        "minecraft:enderman" => match (m.extra.carried, registry) {
            (Some(state), Some(registry)) => {
                map.insert("carriedBlockState".into(), state_to_nbt(registry, state));
            }
            // Nothing carries a block state out without the registry, and a
            // caller without one keeps what the loaded compound had, so the
            // key is only dropped when a registry could have written it.
            (Some(_), None) | (None, None) => {}
            (None, Some(_)) => {
                map.remove(&"carriedBlockState".into());
            }
        },
        "minecraft:shulker" => {
            map.insert("AttachFace".into(), Tag::Byte(m.extra.variant3 as i8));
            map.insert("Peek".into(), Tag::Byte(m.extra.counter as i8));
            map.insert("Color".into(), Tag::Byte(m.extra.color as i8));
        }
        // HappyGhast.addAdditionalSaveData: the still timeout (`serverStillTimeout`,
        // kept in `counter`), which vanilla writes even at 0.
        "minecraft:happy_ghast" => {
            map.insert("still_timeout".into(), Tag::Int(m.extra.counter));
        }
        // GlowSquid.addAdditionalSaveData.
        "minecraft:glow_squid" => {
            map.insert(
                "DarkTicksRemaining".into(),
                Tag::Int(crate::mobs::aquatic::dark_ticks(m)),
            );
        }
        // Phantom.addAdditionalSaveData.
        "minecraft:phantom" => {
            map.insert("size".into(), Tag::Int(m.extra.size));
        }
        "minecraft:slime" | "minecraft:magma_cube" => {
            map.insert("Size".into(), Tag::Int(m.extra.size.max(1) - 1));
            map.insert("wasOnGround".into(), Tag::Byte(i8::from(m.extra.flag2)));
        }
        // Zombie and Skeleton.addAdditionalSaveData (a husk keeps Zombie's):
        // `ConversionTracker`'s conversion and affliction times, -1 when it is
        // not converting or not afflicted.
        "minecraft:zombie" | "minecraft:husk" => {
            for (key, value) in [
                ("DrownedConversionTime", conversion_time(m)),
                ("InWaterTime", m.extra.counter),
            ] {
                map.entry(key.into()).or_insert(Tag::Int(value));
            }
        }
        "minecraft:skeleton" => {
            for (key, value) in [
                ("StrayConversionTime", conversion_time(m)),
                ("FreezingTime", m.extra.counter),
            ] {
                map.entry(key.into()).or_insert(Tag::Int(value));
            }
        }
        // Drowned.convertsInWater is false, so its tracker never ticks: vanilla
        // saves -1 for both.
        "minecraft:drowned" => {
            for (key, value) in [
                ("DrownedConversionTime", Tag::Int(-1)),
                ("InWaterTime", Tag::Int(-1)),
            ] {
                map.entry(key.into()).or_insert(value);
            }
        }
        // Sheep.addAdditionalSaveData: its wool as it is now.
        "minecraft:sheep" => {
            map.insert("Color".into(), Tag::Byte((m.extra.color & 15) as i8));
            map.insert("Sheared".into(), Tag::Byte(i8::from(m.extra.sheared)));
        }
        // CopperGolem.addAdditionalSaveData.
        "minecraft:copper_golem" => {
            map.insert(
                "next_weather_age".into(),
                Tag::Long(m.extra.vector[0] as i64),
            );
            map.insert(
                "weather_state".into(),
                string(crate::mobs::copper_golem::WEATHER[m.extra.variant.min(3) as usize]),
            );
        }
        // Ghast.addAdditionalSaveData; Horizon's ghast fires power 1 whatever
        // is saved, so a loaded value is kept as is.
        "minecraft:ghast" => {
            map.entry("ExplosionPower".into()).or_insert(Tag::Byte(1));
        }
        _ => {}
    }
    // Zombie.addAdditionalSaveData: CanBreakDoors.
    if crate::mobs::doors::is_zombie(m.species()) {
        map.insert(
            "CanBreakDoors".into(),
            Tag::Byte(i8::from(m.extra.can_break_doors)),
        );
        // ConversionTracker.addAdditionalSaveData reads isAfflicted at save
        // time. Zombie supplies its cached isEyeInFluid(WATER), even with NoAI.
        if !m.fluids.eye_in_water {
            map.insert("InWaterTime".into(), Tag::Int(-1));
        }
    }
    if matches!(
        id,
        "minecraft:zombie"
            | "minecraft:husk"
            | "minecraft:drowned"
            | "minecraft:zombie_villager"
            | "minecraft:zombified_piglin"
            | "minecraft:piglin"
            | "minecraft:zoglin"
    ) {
        map.insert("IsBaby".into(), Tag::Byte(i8::from(m.is_baby())));
    }
    // NeutralMob.addPersistentAngerSaveData; the bee saves its own.
    if matches!(
        id,
        "minecraft:iron_golem"
            | "minecraft:wolf"
            | "minecraft:polar_bear"
            | "minecraft:zombified_piglin"
            | "minecraft:enderman"
    ) {
        map.entry("anger_end_time".into()).or_insert(Tag::Long(-1));
    }
    if m.extra.attributes.is_some() {
        put_attributes(map, m);
    }
    // The attribute their load makes, which AttributeMap.pack then saves:
    // Zombie and Piglin.setBaby (even for an adult) the speed, Rabbit.setVariant
    // the attack damage.
    use crate::mobs::finalize::Attr;
    // Every other living entity's first tick makes the movement_speed instance
    // (LivingEntity.removeFrost reads it through getAttribute).
    let made = if species == crate::mobs::Species::Rabbit {
        vec![Attr::MovementSpeed, Attr::AttackDamage]
    } else {
        vec![Attr::MovementSpeed]
    };
    for attr in made {
        let base = crate::mobs::finalize::base_value(m, attr);
        let mut list = match map.get(&"attributes".into()) {
            Some(Tag::List(l)) => l.clone(),
            _ => Vec::new(),
        };
        let attr_id = format!("minecraft:{}", attr.name());
        if !list
            .iter()
            .any(|t| text(t.get("id")).as_deref() == Some(attr_id.as_str()))
        {
            list.push(Tag::compound([
                ("id", string(&attr_id)),
                ("base", Tag::Double(base.to_bits())),
            ]));
            map.insert("attributes".into(), Tag::List(list));
        }
    }
    // Mob.addAdditionalSaveData: the chances that are not the default.
    if let Some(chances) = &m.extra.drop_chances {
        let set: Vec<(&str, Tag)> = crate::mobs::drops::SLOTS
            .iter()
            .zip(chances.iter())
            .filter(|(_, c)| **c != crate::mobs::drops::DEFAULT_DROP_CHANCE)
            .map(|(slot, c)| (*slot, Tag::Float(c.to_bits())))
            .collect();
        if set.is_empty() {
            map.remove(&"drop_chances".into());
        } else {
            map.insert("drop_chances".into(), Tag::compound(set));
        }
    }
    // SpellcasterIllager.addAdditionalSaveData: save the live casting timer,
    // including zero and negative values, rather than stale loaded NBT.
    if matches!(m.species(), S::Evoker | S::Illusioner) {
        map.insert("SpellTicks".into(), Tag::Int(m.extra.counter));
    }
    crate::mobs::raider::write_nbt(m, map);
    crate::mobs::riders::write_nbt(m, map);
    // LivingEntity equipment: what the mob holds and wears now.
    {
        let mut equipment = m.extra.equipment.clone();
        let head = horizon_items::equipment::HEAD;
        // Held and worn items are saved with their enchantments and trim.
        for slot in horizon_items::equipment::MAINHAND..=head {
            equipment[slot] = crate::mobs::finalize::item_stack(m, slot);
        }
        // The captain's ominous banner keeps its patterns.
        if equipment[head].as_ref().is_some_and(|banner| {
            &*banner.item == crate::mobs::raider::BANNER
                && banner.extra.is_none()
                && crate::mobs::raider::is_raider(m.species())
        }) {
            equipment[head] = Some(crate::mobs::raider::ominous_banner());
        }
        equipment.store(map);
        put_live_attributes(map, &m.extra.live);
    }
    tag
}
/// The live map's instances and the loaded attributes nothing else models,
/// over the saved `attributes` list (`AttributeMap.pack`).
pub fn put_live_attributes(map: &mut Compound, live: &crate::living_attributes::LivingAttributes) {
    let mut list = match map.get(&"attributes".into()) {
        Some(Tag::List(l)) => l.clone(),
        _ => Vec::new(),
    };
    let before = list.clone();
    live.save_into(&mut list);
    if list != before {
        map.insert("attributes".into(), Tag::List(list));
    }
}
/// `AttributeInstance.Packed` of the attributes with a base or permanent
/// modifier finalizeSpawn set (or a command or a load did), with their ids.
pub fn attribute_entries(m: &Mob) -> Vec<(String, Tag)> {
    use crate::mobs::finalize::{self, Attr, Op};
    let Some(a) = m.extra.attributes.as_deref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for attr in Attr::ALL {
        let modifiers: Vec<Tag> = a
            .modifiers
            .iter()
            .filter(|x| x.attr == attr)
            .map(|x| {
                let op = match x.op {
                    Op::AddValue => "add_value",
                    Op::AddMultipliedBase => "add_multiplied_base",
                    Op::AddMultipliedTotal => "add_multiplied_total",
                };
                Tag::compound([
                    ("id", string(&x.id)),
                    ("amount", Tag::Double(x.amount.to_bits())),
                    ("operation", string(op)),
                ])
            })
            .collect();
        if modifiers.is_empty() && !a.bases.iter().any(|(b, _)| *b == attr) {
            continue;
        }
        let id = format!("minecraft:{}", attr.name());
        let mut entry = vec![
            ("id", string(&id)),
            ("base", Tag::Double(finalize::base_value(m, attr).to_bits())),
        ];
        if !modifiers.is_empty() {
            entry.push(("modifiers", Tag::List(modifiers)));
        }
        out.push((id, Tag::compound(entry)));
    }
    out
}
/// `AttributeMap.pack` for the attributes with a base or permanent modifier
/// finalizeSpawn set, over the saved list.
fn put_attributes(map: &mut Compound, m: &Mob) {
    let mut list = match map.get(&"attributes".into()) {
        Some(Tag::List(l)) => l.clone(),
        _ => Vec::new(),
    };
    for (id, entry) in attribute_entries(m) {
        match list
            .iter_mut()
            .find(|t| text(t.get("id")).as_deref() == Some(id.as_str()))
        {
            Some(t) => *t = entry,
            None => list.push(entry),
        }
    }
    map.insert("attributes".into(), Tag::List(list));
}
/// `Brain.Packed`: the memories finalizeSpawn set, over the saved ones.
pub fn put_brain(map: &mut Compound, m: &Mob, dimension: &str) {
    if m.extra.memories.is_empty() {
        return;
    }
    let mut memories = match map.get(&"Brain".into()).and_then(|b| b.get("memories")) {
        Some(Tag::Compound(c)) => c.clone(),
        _ => Compound::new(),
    };
    for &(name, value, ttl) in &m.extra.memories {
        let value = match name {
            "hunted_recently" => Tag::Byte(value as i8),
            // `Unit` memories save an empty compound.
            "is_pregnant" => Tag::Compound(Compound::new()),
            "home" => match m.extra.home {
                Some(p) => Tag::compound([
                    ("dimension", string(dimension)),
                    ("pos", Tag::IntArray(p.to_vec())),
                ]),
                None => continue,
            },
            _ => Tag::Int(value as i32),
        };
        let mut memory = vec![("value", value)];
        if ttl != i64::MAX {
            memory.push(("ttl", Tag::Long(ttl)));
        }
        memories.insert(
            format!("minecraft:{name}").as_str().into(),
            Tag::compound(memory),
        );
    }
    map.insert(
        "Brain".into(),
        Tag::compound([("memories", Tag::Compound(memories))]),
    );
}
/// `CustomName`: a text component saved as a string or a `{text: ..}` compound.
fn custom_name(tag: &Tag) -> Option<String> {
    match tag.get("CustomName")? {
        Tag::String(s) => Some(s.to_string_lossy()),
        t @ Tag::Compound(_) => match t.get("text") {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        },
        _ => None,
    }
}
pub use horizon_client_core::component::name_text;
/// Mob.finalizeSpawn's follow range bonus in the saved attributes.
#[cfg(test)]
pub(crate) fn spawn_bonus(tag: &Tag) -> Option<f64> {
    let Some(Tag::List(attrs)) = tag.get("attributes") else {
        return None;
    };
    let range = attrs
        .iter()
        .find(|a| text(a.get("id")).as_deref() == Some("minecraft:follow_range"))?;
    let Some(Tag::List(modifiers)) = range.get("modifiers") else {
        return None;
    };
    let bonus = modifiers
        .iter()
        .find(|x| text(x.get("id")).as_deref() == Some("minecraft:random_spawn_bonus"))?;
    float(bonus.get("amount"))
}
/// `Mob.readAdditionalSaveData`: negative radii retain the live homePosition.
/// Used after the command adapter restores runtime AI, as well as fresh load.
pub fn read_mob_home(m: &mut Mob, tag: &Tag) {
    let radius = match tag.get("home_radius") {
        Some(Tag::Float(v)) => Some(f32::from_bits(*v).floor() as i32),
        Some(Tag::Double(v)) => Some(f64::from_bits(*v).floor() as i32),
        other => number(other).map(|v| v as i32),
    }
    .unwrap_or(-1);
    let pos = if radius >= 0 {
        home_block_pos(tag.get("home_pos")).unwrap_or([0; 3])
    } else {
        m.ai.home_position()
    };
    m.ai.restore_home(pos, radius);
}
/// `BlockPos.CODEC` through NbtOps.getIntStream: numeric collection entries
/// use Java Number's int conversion (floating entries truncate, unlike getIntOr above).
fn home_block_pos(tag: Option<&Tag>) -> Option<[i32; 3]> {
    match tag? {
        Tag::IntArray(v) => v.as_slice().try_into().ok(),
        Tag::ByteArray(v) if v.len() == 3 => Some([
            i32::from(v[0] as i8),
            i32::from(v[1] as i8),
            i32::from(v[2] as i8),
        ]),
        Tag::LongArray(v) if v.len() == 3 => Some([v[0] as i32, v[1] as i32, v[2] as i32]),
        Tag::List(v) if v.len() == 3 => {
            let int = |t: &Tag| match t {
                Tag::Float(v) => Some(f32::from_bits(*v) as i32),
                Tag::Double(v) => Some(f64::from_bits(*v) as i32),
                other => number(Some(other)).map(|v| v as i32),
            };
            Some([int(&v[0])?, int(&v[1])?, int(&v[2])?])
        }
        _ => None,
    }
}
/// The mob a saved compound describes. `registry` is what reads a field holding
/// a block state (an enderman's `carriedBlockState`); without one, such a field
/// is left out, as it was before it was modelled.
pub fn mob_from_nbt(
    tag: &Tag,
    types: &HashMap<String, Arc<MobType>>,
    id: u32,
    registry: Option<&BlockRegistry>,
) -> Option<Mob> {
    let kind = types.get(&self::kind(tag)?)?.clone();
    let position = position(tag)?;
    let [yaw, pitch] = rotation(tag);
    let mut m = Mob::new(id, kind, position, yaw);
    m.velocity = motion(tag);
    m.head_pitch = pitch;
    read_mob_home(&mut m, tag);
    m.on_ground = flag(tag, "OnGround");
    m.fall_distance = float(tag.get("fall_distance")).unwrap_or(0.0);
    // LivingEntity.readAdditionalSaveData: the saved attribute bases, then
    // the health, the max health when there is none.
    if let Some(Tag::List(attrs)) = tag.get("attributes") {
        for a in attrs {
            let (Some(id), Some(Tag::Double(bits))) = (text(a.get("id")), a.get("base")) else {
                continue;
            };
            let v = f64::from_bits(*bits);
            // A mount's rolled health and speed stay attribute bases, which
            // the server syncs (its hearts over the rider's hotbar).
            let equine = crate::mobs::equine::is_equine(m.species());
            match id.as_str() {
                "minecraft:max_health" if equine => crate::mobs::finalize::set_base(
                    &mut m,
                    crate::mobs::finalize::Attr::MaxHealth,
                    v,
                ),
                "minecraft:movement_speed" if equine => crate::mobs::finalize::set_base(
                    &mut m,
                    crate::mobs::finalize::Attr::MovementSpeed,
                    v,
                ),
                // A saved base other than the type's default is the
                // attribute's base, which `/attribute ... base get` reads.
                "minecraft:max_health"
                    if crate::mobs::finalize::default_value(
                        &m,
                        crate::mobs::finalize::Attr::MaxHealth,
                    ) != Some(v) =>
                {
                    crate::mobs::finalize::set_base(
                        &mut m,
                        crate::mobs::finalize::Attr::MaxHealth,
                        v,
                    )
                }
                "minecraft:max_health" => m.attrs.max_health = v as f32,
                "minecraft:movement_speed"
                    if crate::mobs::finalize::default_value(
                        &m,
                        crate::mobs::finalize::Attr::MovementSpeed,
                    ) != Some(v) =>
                {
                    crate::mobs::finalize::set_base(
                        &mut m,
                        crate::mobs::finalize::Attr::MovementSpeed,
                        v,
                    )
                }
                "minecraft:movement_speed" => m.attrs.speed = v,
                "minecraft:attack_damage"
                    if crate::mobs::finalize::default_value(
                        &m,
                        crate::mobs::finalize::Attr::AttackDamage,
                    ) != Some(v) =>
                {
                    crate::mobs::finalize::set_base(
                        &mut m,
                        crate::mobs::finalize::Attr::AttackDamage,
                        v,
                    )
                }
                "minecraft:attack_damage" => m.attrs.attack_damage = v as f32,
                "minecraft:attack_knockback" => m.attrs.attack_knockback = v,
                "minecraft:flying_speed"
                    if crate::mobs::finalize::default_value(
                        &m,
                        crate::mobs::finalize::Attr::FlyingSpeed,
                    ) != Some(v) =>
                {
                    crate::mobs::finalize::set_base(
                        &mut m,
                        crate::mobs::finalize::Attr::FlyingSpeed,
                        v,
                    )
                }
                _ => {}
            }
        }
    }
    // `AttributeMap.apply`: the saved permanent modifiers of the attributes the
    // game models (the follow range bonus finalizeSpawn made among them), and
    // the other saved attributes for the live map.
    if let Some(Tag::List(attrs)) = tag.get("attributes") {
        use crate::mobs::finalize::{Attr, Op, add_modifier};
        for a in attrs {
            let Some(attr) = text(a.get("id")).and_then(|id| {
                Attr::ALL
                    .into_iter()
                    .find(|x| *x != Attr::Armor && x.id() == id)
            }) else {
                continue;
            };
            let Some(Tag::List(modifiers)) = a.get("modifiers") else {
                continue;
            };
            for x in modifiers {
                let op = match text(x.get("operation")).as_deref() {
                    Some("add_value") => Op::AddValue,
                    Some("add_multiplied_base") => Op::AddMultipliedBase,
                    Some("add_multiplied_total") => Op::AddMultipliedTotal,
                    _ => continue,
                };
                if let (Some(id), Some(amount)) = (text(x.get("id")), float(x.get("amount"))) {
                    add_modifier(&mut m, attr, id, amount, op);
                }
            }
        }
        // An attribute the type lacks is skipped (`AttributeMap.apply`).
        let kind = m.kind.clone();
        m.extra.live.load(tag.get("attributes"), |id| {
            kind.attributes.contains_key(id)
                && !Attr::ALL.iter().any(|a| *a != Attr::Armor && a.id() == id)
        });
    }
    m.health = float(tag.get("Health")).map_or(m.attrs.max_health, |h| h as f32);
    m.absorption = float(tag.get("AbsorptionAmount"))
        .map_or(0.0, |h| h as f32)
        .max(0.0);
    // LivingEntity.setHealth clamps to the max health: a horse or llama is made
    // with the type's and saved before it drops to the one it rolled. (Only
    // the base is read here, so the clamp would cut a leader zombie's bonus.)
    if crate::mobs::equine::is_equine(m.species()) {
        m.health = m.health.min(m.attrs.max_health);
    }
    m.hurt_time = number(tag.get("HurtTime")).unwrap_or(0) as i32;
    // What it holds (equipment.mainhand / offhand), when saved.
    if let Some(eq) = tag.get("equipment") {
        // The hands, armour, body and saddle: the one `equipment` codec.
        m.extra.equipment = horizon_items::equipment::Equipment::read(eq);
        // Their enchantments and trims.
        // (`gear` slot order: hands, then head to feet.)
        use horizon_items::equipment::{CHEST, FEET, HEAD, LEGS, MAINHAND, OFFHAND};
        for (index, slot) in [MAINHAND, OFFHAND, HEAD, CHEST, LEGS, FEET]
            .into_iter()
            .enumerate()
        {
            if let Some(stack) = m.extra.slot_stack(slot) {
                let levels = stack.enchantments().to_vec();
                let trim = stack.data().and_then(|d| d.trim.clone());
                crate::mobs::gear::set(&mut m, index, levels);
                crate::mobs::gear::set_trim(&mut m, index, trim);
            }
        }
    } else {
        // Registration hands out default weapons (finalizeSpawn does in
        // vanilla); a loaded mob holds only what its file lists.
        m.extra.set_main_hand(None);
    }
    m.death_time = number(tag.get("DeathTime")).unwrap_or(0) as i32;
    // Mob.readAdditionalSaveData overrides what the constructor set (copper
    // golems and elder guardians start persistent).
    m.persistent = flag(tag, "PersistenceRequired");
    m.extra.death_loot_table = text(tag.get("DeathLootTable"))
        .and_then(|t| horizon_client_core::identifier::normalize_id(&t));
    m.extra.death_loot_seed = number(tag.get("DeathLootTableSeed")).unwrap_or(0);
    // `NeutralMob.readPersistentAngerSaveData`: the anger end time and the
    // target come back with the mob.
    if persistent_anger(m.species()) {
        m.extra.anger_end = number(tag.get("anger_end_time")).unwrap_or(-1);
        // Anger at an entity other than the player is anger at a mob that may
        // not be here (`EntityReference`): it still is not `isAngryAtAllPlayers`.
        m.extra.angry_at = match uuid_from_tag(tag.get("angry_at")) {
            Some(uuid) if uuid == player_uuid() => Some(crate::mobs::Who::Player),
            Some(_) => Some(crate::mobs::Who::Mob(0)),
            None => None,
        };
    }
    // `Mannequin.readAdditionalSaveData`: the profile comes back with the skin, then
    // validated pose, `immovable`, `hide_description` and `description` (`updateDescription`).
    if m.species() == crate::mobs::Species::Mannequin {
        m.profile = tag.get("profile").and_then(profile_from_nbt);
        // Mannequin.POSE_CODEC accepts only these Avatar poses; an invalid
        // or absent value defaults to standing before the live dimensions read it.
        m.extra.avatar_pose = match text(tag.get("pose")).as_deref() {
            Some("crouching") => crate::pose::Pose::Crouching,
            Some("swimming") => crate::pose::Pose::Swimming,
            Some("fall_flying") => crate::pose::Pose::FallFlying,
            Some("sleeping") => crate::pose::Pose::Sleeping,
            _ => crate::pose::Pose::Standing,
        };
        m.extra.immovable = flag(tag, "immovable");
        m.extra.description = if flag(tag, "hide_description") {
            crate::mobs::MannequinDescription::Hidden
        } else {
            crate::mobs::MannequinDescription::from_component(Some(
                &tag.get("description")
                    .cloned()
                    .or_else(|| crate::mobs::MannequinDescription::Default.component())
                    .unwrap_or(Tag::End),
            ))
        };
    }
    m.extra.pick_up_loot = flag(tag, "CanPickUpLoot");
    m.custom_name = custom_name(tag);
    m.custom_name_text = tag.get("CustomName").map(name_text);
    m.extra.left_handed = flag(tag, "LeftHanded");
    if let Some(Tag::Compound(c)) = tag.get("drop_chances") {
        let mut chances = [crate::mobs::drops::DEFAULT_DROP_CHANCE; 8];
        for (chance, slot) in chances.iter_mut().zip(crate::mobs::drops::SLOTS) {
            if let Some(Tag::Float(v)) = c.get(&slot.into()) {
                *chance = f32::from_bits(*v);
            }
        }
        m.extra.drop_chances = Some(Box::new(chances));
    }
    if saves_age(m.species()) {
        m.extra.age = tag.get("Age").and_then(int_value).unwrap_or(0);
        m.extra.forced_age = tag.get("ForcedAge").and_then(int_value).unwrap_or(0);
        m.extra.age_locked = flag(tag, "AgeLocked");
    }
    if is_animal(&m.kind.id) {
        m.extra.in_love = tag.get("InLove").and_then(int_value).unwrap_or(0);
        // Animal.readAdditionalSaveData: the player who fed it into love.
        m.extra.love_cause = crate::save::uuid_from_tag(tag.get("LoveCause"));
    }
    m.name_visible = flag(tag, "CustomNameVisible");
    m.no_ai = flag(tag, "NoAI");
    m.no_gravity = flag(tag, "NoGravity");
    m.glowing = flag(tag, "Glowing");
    m.ticks_frozen = number(tag.get("TicksFrozen")).unwrap_or(0) as i32;
    // LivingEntity.readAdditionalSaveData: the saved `active_effects`, the last
    // entry of an effect winning.
    if let Some(Tag::List(effects)) = tag.get("active_effects") {
        for e in effects {
            let Some(def) = text(e.get("id")).and_then(|id| crate::brewing::effect(&id)) else {
                continue;
            };
            let effect = crate::brewing::namespaced(def.id);
            let duration = number(e.get("duration")).unwrap_or(0) as i32;
            let amplifier = number(e.get("amplifier")).unwrap_or(0) as i32;
            m.load_effect(effect, duration, amplifier);
            m.extra.effect_chains.retain(|e| e.id != effect);
            if let Some(hidden) = e.get("hidden_effect") {
                m.extra
                    .effect_chains
                    .push(hidden_effect_from_nbt(effect, hidden));
            }
            // Details.CODEC: `show_particles` defaults to true.
            m.extra.hidden_effects.retain(|h| *h != effect);
            if number(e.get("show_particles")) == Some(0) {
                m.extra.hidden_effects.push(effect);
            }
            // Details.CODEC: `ambient` defaults to false.
            m.extra.ambient_effects.retain(|h| *h != effect);
            if number(e.get("ambient")).is_some_and(|a| a != 0) {
                m.extra.ambient_effects.push(effect);
            }
        }
    }
    m.silent = flag(tag, "Silent");
    m.invulnerable_flag = flag(tag, "Invulnerable");
    if m.species() == crate::mobs::Species::Bat {
        // Bat.readAdditionalSaveData: BatFlags bit 0, resting.
        m.extra.flag = number(tag.get("BatFlags")).unwrap_or(0) & 1 != 0;
    }
    if m.species() == crate::mobs::Species::Creeper {
        m.extra.charged = flag(tag, "powered");
        // Creeper.readAdditionalSaveData: `ignited` lights the fuse.
        m.extra.flag = flag(tag, "ignited");
        m.max_swell = number(tag.get("Fuse")).unwrap_or(30) as i16 as i32;
        // `getByteOr("ExplosionRadius", 3)`: any numeric tag, as a byte.
        m.extra.explosion_radius = number(tag.get("ExplosionRadius")).unwrap_or(3) as i8;
    }
    // InventoryCarrier.readInventoryFromTag, using the existing occupied-stack model.
    let carrier_slots = match m.species() {
        crate::mobs::Species::Pillager => 5,
        crate::mobs::Species::WanderingTrader => 8,
        _ => 0,
    };
    if carrier_slots != 0
        && let Some(Tag::List(items)) = tag.get("Inventory")
    {
        m.extra.inventory = items
            .iter()
            .filter_map(crate::save::item_from_nbt)
            .take(carrier_slots)
            .collect();
    }
    if matches!(
        m.species(),
        crate::mobs::Species::Piglin
            | crate::mobs::Species::PiglinBrute
            | crate::mobs::Species::Hoglin
    ) {
        m.extra.flag2 = flag(tag, "IsImmuneToZombification");
        m.extra.counter2 = number(tag.get("TimeInOverworld")).unwrap_or(0) as i32;
        if m.species() == crate::mobs::Species::Hoglin {
            m.extra.cannot_be_hunted = flag(tag, "CannotBeHunted");
        }
        if m.species() == crate::mobs::Species::Piglin
            && let Some(Tag::List(items)) = tag.get("Inventory")
        {
            // InventoryCarrier.readInventoryFromTag (eight slots).
            m.extra.inventory = items
                .iter()
                .filter_map(crate::save::item_from_nbt)
                .take(8)
                .collect();
        }
    }
    if matches!(
        m.species(),
        crate::mobs::Species::Pig | crate::mobs::Species::Strider
    ) {
        // The old `Saddle` flag, when `equipment` held none.
        if flag(tag, "Saddle") && m.saddle_item().is_none() {
            m.set_saddle(Some(crate::items::ItemStack::new("minecraft:saddle", 1)));
        }
    }
    if crate::mobs::equine::is_equine(m.species()) {
        m.extra.tame = flag(tag, "Tame");
        m.extra.variant3 = number(tag.get("Temper")).unwrap_or(0).clamp(0, 100) as u8;
        m.extra.bred = flag(tag, "Bred");
        m.extra.owner = crate::save::uuid_from_tag(tag.get("Owner"));
        let v = number(tag.get("Variant")).unwrap_or(0) as i32;
        m.extra.variant = (v & 0xFF).clamp(0, 6) as u8;
        m.extra.variant2 = ((v >> 8) & 0xFF).clamp(0, 4) as u8;
        if crate::mobs::llama::is_llama(m.species()) {
            crate::mobs::llama::from_nbt(&mut m, tag);
        }
        m.extra.chested = flag(tag, "ChestedHorse");
        if m.extra.chested {
            m.extra.items = crate::save::decode_items(tag.get("Items"), 15);
        }
        if let Some(Tag::List(attrs)) = tag.get("attributes") {
            for a in attrs {
                let (Some(id), Some(Tag::Double(bits))) = (text(a.get("id")), a.get("base")) else {
                    continue;
                };
                if id == "minecraft:jump_strength" {
                    m.extra.vector[0] = f64::from_bits(*bits);
                }
            }
        }
        if m.extra.vector[0] <= 0.0 {
            m.extra.vector[0] = 0.7;
        }
    }
    if crate::mobs::finalize::saves_variant_name(m.species())
        && let Some(v) = text(tag.get("variant"))
    {
        m.extra.variant = crate::mobs::finalize::variant_index(m.species(), &v);
    }
    if m.species() == crate::mobs::Species::Chicken
        && let Some(t) = number(tag.get("EggLayTime"))
    {
        m.extra.egg_time = t as i32;
    }
    if let Some(v) = text(tag.get("sound_variant")) {
        let v = v.trim_start_matches("minecraft:");
        let sounds = crate::mobs::finalize::sound_variants(m.species());
        m.extra.sound_variant = sounds.iter().position(|n| *n == v).map(|i| i as u8);
    }
    match m.species() {
        crate::mobs::Species::Fox => {
            m.extra.variant = u8::from(text(tag.get("Type")).as_deref() == Some("snow"));
            // Fox.readAdditionalSaveData: clearTrusted, then addTrustedEntity for
            // each of "Trusted".
            crate::mobs::fox::clear_trusted(&mut m);
            if let Some(Tag::List(trusted)) = tag.get("Trusted") {
                for uuid in trusted
                    .iter()
                    .filter_map(|t| crate::save::uuid_from_tag(Some(t)))
                {
                    crate::mobs::fox::add_trusted_entity(&mut m, uuid);
                }
            }
            // Fox.readAdditionalSaveData: the sleeping, sitting and crouching flags.
            for (key, bit) in [
                ("Sleeping", crate::mobs::fox::SLEEPING),
                ("Sitting", crate::mobs::fox::SITTING),
                ("Crouching", crate::mobs::fox::CROUCHING),
            ] {
                if flag(tag, key) {
                    m.extra.ints[0] |= bit;
                }
            }
        }
        // MushroomCow.readAdditionalSaveData: a missing or unknown name is
        // Variant.DEFAULT, red; stew_effects decodes like the component's list.
        crate::mobs::Species::Mooshroom => {
            m.extra.variant = u8::from(text(tag.get("Type")).as_deref() == Some("brown"));
            if let Some(Tag::List(list)) = tag.get("stew_effects") {
                m.extra.stew_effects = list
                    .iter()
                    .map(|entry| {
                        Some((
                            Arc::from(text(entry.get("id"))?.as_str()),
                            number(entry.get("duration")).unwrap_or(160) as i32,
                        ))
                    })
                    .collect();
            }
        }
        crate::mobs::Species::Rabbit => {
            // Rabbit.Variant.byId: an unknown id is brown.
            m.extra.variant = match number(tag.get("RabbitType")).unwrap_or(0) {
                v @ (0..=5 | 99) => v as u8,
                _ => 0,
            };
        }
        crate::mobs::Species::Goat => {
            m.extra.screaming = flag(tag, "IsScreamingGoat");
            let horn = |key| number(tag.get(key)).is_some_and(|v| v == 0);
            m.extra.lost_horns = [horn("HasLeftHorn"), horn("HasRightHorn")];
        }
        _ => {}
    }
    // `Rabbit.readAdditionalSaveData`.
    if m.species() == crate::mobs::Species::Rabbit {
        m.extra.more_carrot_ticks = number(tag.get("MoreCarrotTicks")).unwrap_or(0) as i32;
    }
    // `Ocelot.readAdditionalSaveData`.
    if m.species() == crate::mobs::Species::Ocelot {
        m.extra.trusting = flag(tag, "Trusting");
    }
    if m.species() == crate::mobs::Species::Cat {
        m.extra.owner = crate::save::uuid_from_tag(tag.get("Owner"));
        m.extra.tame = m.extra.owner.is_some();
        m.extra.sitting = flag(tag, "Sitting");
        m.extra.color =
            number(tag.get("CollarColor")).map_or(crate::mobs::tame::RED, |c| (c as u8) & 15);
    }
    if m.species() == crate::mobs::Species::Panda {
        let gene = |key| text(tag.get(key)).map_or(0, |n| crate::mobs::panda::gene_from_name(&n));
        m.extra.variant = gene("MainGene");
        m.extra.variant2 = gene("HiddenGene");
    }
    if m.species() == crate::mobs::Species::Parrot {
        m.extra.owner = crate::save::uuid_from_tag(tag.get("Owner"));
        m.extra.tame = m.extra.owner.is_some();
        m.extra.sitting = flag(tag, "Sitting");
    }
    if m.species() == crate::mobs::Species::Wolf {
        let health = m.health;
        m.extra.owner = crate::save::uuid_from_tag(tag.get("Owner"));
        let tame = m.extra.owner.is_some();
        crate::mobs::tame::set_tame(&mut m, tame);
        m.health = health;
        m.extra.sitting = flag(tag, "Sitting");
        m.extra.color =
            number(tag.get("CollarColor")).map_or(crate::mobs::tame::RED, |c| (c as u8) & 15);
    }
    if matches!(
        m.species(),
        crate::mobs::Species::Villager | crate::mobs::Species::WanderingTrader
    ) {
        read_villager(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::ZombieVillager {
        read_villager(&mut m, tag);
        let xp = number(tag.get("Xp")).unwrap_or(0) as i32;
        if xp != 0 {
            m.extra.trade.get_or_insert_with(Default::default).xp = xp;
        }
        crate::mobs::zombie_villager::read_nbt(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::Dolphin {
        // Dolphin.readAdditionalSaveData: a save that names neither field is an
        // unfed dolphin at full moistness.
        m.extra.flag = flag(tag, "GotFish");
        m.extra.counter = number(tag.get("Moistness")).unwrap_or(2400) as i32;
    }
    if m.species() == crate::mobs::Species::Axolotl {
        restore_axolotl_memories(&mut m, tag);
    }
    read_variant(&mut m, tag);
    m.fire_ticks = number(tag.get("Fire")).unwrap_or(0) as i32;
    // Entity.load: `input.getIntOr("Air", getMaxAirSupply())`, 4800 for a dolphin.
    m.air = number(tag.get("Air")).map_or(m.max_air_supply(), |air| air as i32);
    if m.species() == crate::mobs::Species::Allay {
        if let Some(Tag::List(inv)) = tag.get("Inventory") {
            for stack in inv.iter().filter_map(item_from_nbt) {
                let max = m.kind.inventory_stack_limit(&stack);
                if let Some(st) = m.extra.allay.as_mut() {
                    st.add(stack, max);
                }
            }
        }
        m.extra.egg_time = number(tag.get("DuplicationCooldown")).unwrap_or(0) as i32;
        if let (Some(st), Some(listener)) = (m.extra.allay.as_mut(), tag.get("listener")) {
            st.vibration = crate::sculk::Data::from_nbt(listener);
        }
        let liked = tag
            .get("Brain")
            .and_then(|b| b.get("memories"))
            .and_then(|b| b.get("minecraft:liked_player"));
        m.extra.tame = liked.is_some();
        m.extra.owner = liked
            .and_then(|l| l.get("value"))
            .and_then(|v| crate::save::uuid_from_tag(Some(v)));
        let memory = |key: &str| {
            tag.get("Brain")
                .and_then(|b| b.get("memories"))
                .and_then(|b| b.get(key))
                .and_then(|l| l.get("value"))
        };
        m.extra.ints[2] =
            number(memory("minecraft:liked_noteblock_cooldown_ticks")).unwrap_or(0) as i32;
        if let Some(st) = m.extra.allay.as_mut()
            && let Some(noteblock) = memory("minecraft:liked_noteblock")
            && let Some(Tag::IntArray(p)) = noteblock.get("pos")
            && p.len() == 3
        {
            st.noteblock = Some(crate::mobs::allay::GlobalPos {
                dimension: crate::mobs::allay::dimension_id(
                    &text(noteblock.get("dimension")).unwrap_or_default(),
                ),
                pos: [p[0], p[1], p[2]],
            });
        }
    }
    if m.species() == crate::mobs::Species::Frog
        && tag
            .get("Brain")
            .and_then(|b| b.get("memories"))
            .and_then(|b| b.get("minecraft:is_pregnant"))
            .is_some()
    {
        crate::mobs::frog::set_pregnant(&mut m, true);
    }
    if crate::mobs::camel::is_camel(m.species()) {
        // The tick is a game time: `camel::finish_load` puts it on the mob's
        // clock once the world's time is known.
        let tick = match tag.get("LastPoseTick") {
            Some(Tag::Long(v)) => *v,
            _ => 0,
        };
        crate::mobs::camel::stand_up_instantly(&mut m);
        m.extra.pose_change_tick = tick;
        m.extra.camel_sitting = tick < 0;
        // Camels are always tame (`Camel.register` in Horizon).
        m.extra.tame = true;
    }
    if crate::mobs::nautilus::is_nautilus(m.species()) {
        crate::mobs::nautilus::from_nbt(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::SulfurCube {
        crate::mobs::sulfur_cube::from_nbt(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::Bee {
        crate::mobs::bee::from_nbt(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::Creaking
        && let Some(Tag::IntArray(home)) = tag.get("home_pos")
        && home.len() == 3
    {
        crate::mobs::creaking::set_transient(&mut m, [home[0], home[1], home[2]]);
    }
    if m.species() == crate::mobs::Species::Turtle {
        // Turtle.readAdditionalSaveData: `home_pos` (else where it stands) and `has_egg`.
        m.extra.flag = flag(tag, "has_egg");
        m.extra.home = Some(match tag.get("home_pos") {
            Some(Tag::IntArray(home)) if home.len() == 3 => [home[0], home[1], home[2]],
            _ => [
                m.position.x.floor() as i32,
                m.position.y.floor() as i32,
                m.position.z.floor() as i32,
            ],
        });
    }
    if m.species() == crate::mobs::Species::Wither {
        m.extra.counter = number(tag.get("Invul")).unwrap_or(0) as i32;
    }
    if m.species() == crate::mobs::Species::Sniffer {
        crate::mobs::sniffer::from_nbt(&mut m, tag);
    }
    if m.species() == crate::mobs::Species::CopperGolem {
        // CopperGolem.readAdditionalSaveData: -1 (unset) when missing.
        m.extra.vector[0] = match tag.get("next_weather_age") {
            Some(Tag::Long(v)) => *v as f64,
            _ => -1.0,
        };
        let weather = match tag.get("weather_state") {
            Some(Tag::String(s)) => s.to_string_lossy(),
            _ => String::new(),
        };
        m.extra.variant = crate::mobs::copper_golem::WEATHER
            .iter()
            .position(|w| *w == weather)
            .unwrap_or(0) as u8;
    }
    m.extra.leash_pending = crate::mobs::leash::pending_from_nbt(tag.get("leash"));
    if m.species() == crate::mobs::Species::Warden {
        // The brain starts afresh: a minute before it digs back down.
        crate::mobs::warden::finalize(&mut m, false, &mut Vec::new());
        if let (Some(st), Some(listener)) = (m.extra.warden.as_mut(), tag.get("listener")) {
            st.vibration = crate::sculk::Data::from_nbt(listener);
        }
    }
    // Armadillo.readAdditionalSaveData: keeps the constructor's time when absent.
    if m.species() == crate::mobs::Species::Armadillo
        && let Some(t) = number(tag.get("scute_time"))
    {
        m.extra.egg_time = t as i32;
    }
    if m.species() == crate::mobs::Species::Ravager {
        m.extra.counter = number(tag.get("AttackTick")).unwrap_or(0) as i32;
        m.extra.counter2 = number(tag.get("StunTick")).unwrap_or(0) as i32;
        m.extra.anim_ticks = number(tag.get("RoarTick")).unwrap_or(0) as i32;
    }
    if m.species() == crate::mobs::Species::Endermite {
        m.extra.counter = number(tag.get("Lifetime")).unwrap_or(0) as i32;
    }
    if m.species() == crate::mobs::Species::Vex {
        if let Some(Tag::IntArray(v)) = tag.get("bound_pos")
            && v.len() == 3
        {
            m.extra.home = Some([v[0], v[1], v[2]]);
        }
        m.extra.egg_time = number(tag.get("life_ticks")).map_or(-1, |v| v as i32);
    }
    // Sheep.readAdditionalSaveData: white and unsheared when absent.
    if m.species() == crate::mobs::Species::Sheep {
        m.extra.color = number(tag.get("Color")).map_or(0, |c| (c as u8) & 15);
        m.extra.sheared = flag(tag, "Sheared");
    }
    // GlowSquid.readAdditionalSaveData: not dimmed when absent.
    if m.species() == crate::mobs::Species::GlowSquid {
        crate::mobs::aquatic::set_dark_ticks(
            &mut m,
            number(tag.get("DarkTicksRemaining")).unwrap_or(0) as i32,
        );
    }
    // IronGolem.readAdditionalSaveData: not player created when absent.
    if m.species() == crate::mobs::Species::IronGolem {
        m.extra.player_created = flag(tag, "PlayerCreated");
    }
    // Pufferfish.readAdditionalSaveData: `getIntOr("PuffState", 0)`, at most 2.
    if m.species() == crate::mobs::Species::Pufferfish {
        m.extra.size = number(tag.get("PuffState")).unwrap_or(0).min(2) as i32;
    }
    // Enderman.readAdditionalSaveData: the block it carried; an air state
    // reads as no block at all.
    if m.species() == crate::mobs::Species::Enderman
        && let Some(registry) = registry
    {
        m.extra.carried = tag
            .get("carriedBlockState")
            .and_then(|t| state_from_nbt(registry, t))
            .filter(|&state| !is_air(registry, state));
    }
    // HappyGhast.readAdditionalSaveData: the still timeout, 0 when absent (a fresh
    // ghast, or one saved by a version that did not write it).
    if m.species() == crate::mobs::Species::HappyGhast {
        m.extra.counter = number(tag.get("still_timeout")).unwrap_or(0) as i32;
    }
    if m.species() == crate::mobs::Species::Shulker {
        m.extra.variant3 = number(tag.get("AttachFace")).unwrap_or(0).clamp(0, 5) as u8;
        m.extra.counter = number(tag.get("Peek")).unwrap_or(0).clamp(0, 100) as i32;
        m.extra.color = number(tag.get("Color")).map_or(16, |c| c.clamp(0, 16) as u8);
        m.anim.v[0] = m.extra.counter as f32 * 0.01;
        m.anim.vo[0] = m.anim.v[0];
        // Shulker.setPos: square on its block.
        m.position = horizon_core::geometry::Vec3::new(
            m.position.x.floor() + 0.5,
            (m.position.y + 0.5).floor(),
            m.position.z.floor() + 0.5,
        );
        m.previous = m.position;
    }
    if matches!(
        m.species(),
        crate::mobs::Species::Slime | crate::mobs::Species::MagmaCube
    ) {
        // AbstractCubeMob.readAdditionalSaveData: the size's attributes, then
        // the saved health, the max health when there is none.
        crate::mobs::cube::set_size(&mut m, number(tag.get("Size")).unwrap_or(0) as i32 + 1);
        if let Some(h) = float(tag.get("Health")) {
            m.health = h as f32;
        }
        m.extra.flag2 = flag(tag, "wasOnGround");
    }
    if m.species() == crate::mobs::Species::Phantom {
        // Phantom.readAdditionalSaveData: setPhantomSize(size).
        crate::mobs::phantom::set_size(&mut m, number(tag.get("size")).unwrap_or(0) as i32);
    }
    // Zombie, Piglin and Zoglin.readAdditionalSaveData: IsBaby (the baby
    // zombies of trial spawners).
    if !crate::mobs::finalize::ageable(m.species()) && flag(tag, "IsBaby") {
        crate::mobs::finalize::set_baby(&mut m, true);
    }
    // Zombie.readAdditionalSaveData: setCanBreakDoors.
    if crate::mobs::doors::is_zombie(m.species()) {
        crate::mobs::doors::set_can_break_doors(&mut m, flag(tag, "CanBreakDoors"));
    }
    // Zombie and Skeleton.readAdditionalSaveData: `ConversionTracker` reads its
    // affliction time (0 when absent) and starts a conversion from a saved time
    // other than -1, -1 stopping one (`setConverting(false)`).
    let tracker = match m.species() {
        crate::mobs::Species::Zombie | crate::mobs::Species::Husk => {
            Some(("InWaterTime", "DrownedConversionTime"))
        }
        crate::mobs::Species::Skeleton => Some(("FreezingTime", "StrayConversionTime")),
        _ => None,
    };
    if let Some((affliction, conversion)) = tracker {
        m.extra.counter = number(tag.get(affliction)).unwrap_or(0) as i32;
        let time = number(tag.get(conversion)).unwrap_or(-1) as i32;
        // Any other time starts a conversion (`startConversion`), a negative
        // one spent on the next tick.
        m.extra.counter2 = if time == -1 { 0 } else { time.max(0) + 1 };
    }
    // SpellcasterIllager.readAdditionalSaveData: this is the same timer used
    // by casting goals and the existing customServerAiStep countdown.
    if matches!(
        m.species(),
        crate::mobs::Species::Evoker | crate::mobs::Species::Illusioner
    ) {
        m.extra.counter = number(tag.get("SpellTicks")).unwrap_or(0) as i32;
    }
    crate::mobs::raider::read_nbt(&mut m, tag);
    crate::mobs::riders::read_nbt(&mut m, tag);
    // A mob saved mid-death finishes dying; one saved dead is dropped.
    if m.health <= 0.0 && m.death_time == 0 {
        return None;
    }
    // DeathTime only counts while dead.
    if m.health > 0.0 {
        m.death_time = 0;
    }
    Some(m)
}

/// `ItemEntity`: Health, Age, PickupDelay, Item, Thrower.
pub fn item_entity_to_nbt(e: &ItemEntity, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let base = Base {
        id: "minecraft:item",
        uuid,
        position: e.position,
        motion: e.velocity,
        rotation: e.rotation,
        on_ground: e.on_ground,
        fall_distance: e.fall_distance,
    };
    let mut tag = base.non_mob_tag(raw);
    for (key, value) in [
        ("Item", item_to_nbt(&e.stack)),
        ("Health", Tag::Short(e.health as i16)),
        ("Fire", Tag::Short(e.fire_ticks as i16)),
        (
            "Age",
            Tag::Short(e.age.clamp(i16::MIN as i32, i16::MAX as i32) as i16),
        ),
        (
            "PickupDelay",
            Tag::Short(
                e.pickup_delay
                    .clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
            ),
        ),
    ] {
        put(&mut tag, key, value);
    }
    if let Some(thrower) = e.thrower {
        put(&mut tag, "Thrower", Tag::IntArray(thrower.to_vec()));
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"Thrower".into());
    }
    if let Some(target) = e.target {
        put(&mut tag, "Owner", Tag::IntArray(target.to_vec()));
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"Owner".into());
    }
    tag
}
/// `PrimedTnt.readAdditionalSaveData`: `explosion_power`, 4 by default and
/// clamped to 0..128.
pub fn tnt_power(tag: &Tag) -> f32 {
    match tag.get("explosion_power") {
        Some(Tag::Float(bits)) => f32::from_bits(*bits).clamp(0.0, 128.0),
        _ => 4.0,
    }
}

/// `PrimedTnt.addAdditionalSaveData`: its `fuse` and, when it is not the
/// default, its `explosion_power`, the encoded live block state, and its
/// `owner` (`EntityReference.store`). With no live owner, a loaded `owner`
/// stays as the raw compound has it.
#[allow(clippy::too_many_arguments)]
pub fn tnt_to_nbt(
    position: Vec3,
    motion: Vec3,
    on_ground: bool,
    fuse: i32,
    power: f32,
    block_state: Tag,
    rotation: [f32; 2],
    uuid: [i32; 4],
    owner: Option<[i32; 4]>,
    raw: Option<&Tag>,
) -> Tag {
    let mut tag = Base {
        id: "minecraft:tnt",
        uuid,
        position,
        motion,
        rotation,
        on_ground,
        fall_distance: 0.0,
    }
    .tag(raw);
    put(&mut tag, "fuse", Tag::Short(fuse as i16));
    if power != 4.0 {
        put(&mut tag, "explosion_power", Tag::Float(power.to_bits()));
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"explosion_power".into());
    }
    put(&mut tag, "block_state", block_state);
    if let Some(owner) = owner {
        put(&mut tag, "owner", Tag::IntArray(owner.to_vec()));
    }
    tag
}
/// `PrimedTnt.readAdditionalSaveData`: `EntityReference.read(input, "owner")`.
pub fn tnt_owner(tag: &Tag) -> Option<[i32; 4]> {
    uuid_from_tag(tag.get("owner"))
}
/// `EvokerFangs.addAdditionalSaveData`: always save the live warmup counter.
/// Known Owner UUIDs use `EntityReference.store` encoding; unresolved
/// loaded Owner data remains in the raw compound.
pub fn evoker_fangs_to_nbt(
    position: Vec3,
    yaw: f32,
    warmup: i32,
    owner: Option<[i32; 4]>,
    uuid: [i32; 4],
    raw: Option<&Tag>,
) -> Tag {
    let mut tag = Base {
        id: "minecraft:evoker_fangs",
        uuid,
        position,
        motion: raw.map(motion).unwrap_or(Vec3::ZERO),
        rotation: [yaw, raw.map(rotation).unwrap_or([0.0; 2])[1]],
        on_ground: raw.is_some_and(|t| flag(t, "OnGround")),
        fall_distance: 0.0,
    }
    .tag(raw);
    put(&mut tag, "Warmup", Tag::Int(warmup));
    if let Some(owner) = owner {
        put(&mut tag, "Owner", Tag::IntArray(owner.to_vec()));
    }
    tag
}

/// `UUIDUtil.CODEC` via `NbtOps.getIntStream`: boxed Number conversions,
/// with the first-four partial value retained by `TagValueInput.read`.
fn item_uuid_from_tag(tag: Option<&Tag>) -> Option<[i32; 4]> {
    let values = horizon_world::datafix::dynamic::as_int_stream(tag)?;
    values.get(..4)?.try_into().ok()
}

/// `ItemEntity.readAdditionalSaveData`: Owner and Thrower use the numeric UUID codec.
pub fn item_entity_from_nbt(tag: &Tag, id: u32, bob_offset: f32) -> Option<ItemEntity> {
    let stack = item_from_nbt(tag.get("Item")?)?;
    let position = position(tag)?;
    Some(ItemEntity {
        attachment_lifetime: Default::default(),
        fall_distance: float(tag.get("fall_distance")).unwrap_or(0.0),
        rotation: rotation(tag),
        join: Default::default(),
        no_gravity: flag(tag, "NoGravity"),
        health: number(tag.get("Health")).unwrap_or(5) as i32,
        fire_ticks: number(tag.get("Fire")).unwrap_or(0) as i32,
        id,
        stack,
        thrower: item_uuid_from_tag(tag.get("Thrower")),
        target: item_uuid_from_tag(tag.get("Owner")),
        position,
        previous: position,
        velocity: motion(tag),
        on_ground: flag(tag, "OnGround"),
        vertical_collision: false,
        horizontal_collision: false,
        age: number(tag.get("Age")).unwrap_or(0) as i32,
        pickup_delay: number(tag.get("PickupDelay")).unwrap_or(0),
        bob_offset,
        ticks: 0,
        needs_sync: false,
        stuck: Vec3::ZERO,
        was_touching_water: false,
    })
}

/// `OminousItemSpawner.addAdditionalSaveData`: its item (left out when
/// empty) and `spawn_item_after_ticks`. Its tick count is the entity's
/// `tickCount`, which isn't saved: a reloaded spawner counts from 0 again.
pub fn ominous_item_spawner_to_nbt(
    item: &crate::items::ItemStack,
    position: Vec3,
    spawn_item_after: i64,
    uuid: [i32; 4],
    raw: Option<&Tag>,
) -> Tag {
    let base = Base {
        id: "minecraft:ominous_item_spawner",
        uuid,
        position,
        motion: Vec3::ZERO,
        rotation: [0.0, 0.0],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    // `ItemStack.CODEC` can't encode `ItemStack.EMPTY`, so an empty item
    // writes no key.
    if let Tag::Compound(map) = &mut tag {
        map.remove(&"item".into());
    }
    if &*item.item != "minecraft:air" {
        put(&mut tag, "item", item_to_nbt(item));
    }
    put(
        &mut tag,
        "spawn_item_after_ticks",
        Tag::Long(spawn_item_after),
    );
    tag
}
/// `OminousItemSpawner.readAdditionalSaveData`: the item, where it floats
/// and `spawn_item_after_ticks` (0 when absent). None without an item:
/// such a spawner would spawn nothing.
pub fn ominous_item_spawner_from_nbt(tag: &Tag) -> Option<(crate::items::ItemStack, Vec3, i64)> {
    // `readAdditionalSaveData`: `ItemStack.EMPTY` when it has none.
    let item = tag
        .get("item")
        .and_then(item_from_nbt)
        .unwrap_or_else(|| crate::items::ItemStack::new("minecraft:air", 1));
    let after = number(tag.get("spawn_item_after_ticks")).unwrap_or(0);
    Some((item, position(tag)?, after))
}

/// `ExperienceOrb`: Health, Age, Value, Count.
pub fn orb_to_nbt(o: &ExperienceOrb, uuid: [i32; 4]) -> Tag {
    let base = Base {
        id: "minecraft:experience_orb",
        uuid,
        position: o.position,
        motion: o.velocity,
        rotation: o.rotation,
        on_ground: o.on_ground,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(None);
    for (key, value) in [
        ("Health", Tag::Short(o.health as i16)),
        ("Age", Tag::Short(o.age.min(i16::MAX as u32) as i16)),
        ("Value", Tag::Short(o.value.min(i16::MAX as u32) as i16)),
        ("Count", Tag::Int(o.count.min(i32::MAX as u32) as i32)),
    ] {
        put(&mut tag, key, value);
    }
    // Entity.saveWithoutId writes NoGravity only when set.
    if o.no_gravity {
        put(&mut tag, "NoGravity", Tag::Byte(1));
    }
    tag
}
/// One orb with its merged `Count`.
pub fn orbs_from_nbt(tag: &Tag) -> Vec<ExperienceOrb> {
    let Some(position) = position(tag) else {
        return Vec::new();
    };
    let count = number(tag.get("Count")).unwrap_or(1).max(1) as u32;
    let orb = ExperienceOrb {
        attachment_lifetime: Default::default(),
        rotation: rotation(tag),
        join: Default::default(),
        no_gravity: flag(tag, "NoGravity"),
        id: 0,
        position,
        previous: position,
        velocity: motion(tag),
        value: number(tag.get("Value")).unwrap_or(1).max(1) as u32,
        age: number(tag.get("Age")).unwrap_or(0).max(0) as u32,
        on_ground: flag(tag, "OnGround"),
        count,
        ticks: 0,
        health: number(tag.get("Health")).unwrap_or(5) as i32,
    };
    vec![orb]
}

/// `Projectile` + `AbstractArrow.addAdditionalSaveData` of a live arrow;
/// `in_block` is its `inBlockState`, `raw` the compound it was loaded with.
pub fn arrow_to_nbt(
    p: &Projectile,
    uuid: [i32; 4],
    owner: Option<[i32; 4]>,
    in_block: Option<Tag>,
    raw: Option<&Tag>,
) -> Option<Tag> {
    let a = p.arrow.as_deref()?;
    let ProjectileKind::Arrow {
        base_damage,
        critical,
        ..
    } = p.kind
    else {
        return None;
    };
    let base = Base {
        // A spectral arrow is its own entity type.
        id: if a.spectral || &*a.pickup_item.item == "minecraft:spectral_arrow" {
            "minecraft:spectral_arrow"
        } else {
            "minecraft:arrow"
        },
        uuid,
        position: p.position,
        motion: p.velocity,
        rotation: [p.yaw, p.pitch],
        on_ground: false,
        fall_distance: a.fall_distance,
    };
    let mut tag = base.tag(raw);
    for (key, value) in [
        ("Fire", Tag::Short(a.fire_ticks as i16)),
        ("life", Tag::Short(a.life as i16)),
        ("shake", Tag::Byte(a.shake as i8)),
        ("inGround", Tag::Byte(i8::from(a.in_ground))),
        ("pickup", Tag::Byte(a.pickup.byte())),
        ("damage", Tag::Double(base_damage.to_bits())),
        ("crit", Tag::Byte(i8::from(critical))),
        ("PierceLevel", Tag::Byte(a.pierce as i8)),
        ("HasBeenShot", Tag::Byte(i8::from(a.has_been_shot))),
        ("item", item_to_nbt(&a.pickup_item)),
    ] {
        put(&mut tag, key, value);
    }
    if let Tag::Compound(map) = &mut tag {
        map.entry("SoundEvent".into())
            .or_insert_with(|| string("minecraft:entity.arrow.hit"));
        // Projectile: LeftOwner only when set; Owner as the UUID.
        map.remove(&"LeftOwner".into());
        map.remove(&"inBlockState".into());
        map.remove(&"weapon".into());
    }
    if let Some(weapon) = &a.fired_from_weapon {
        put(&mut tag, "weapon", item_to_nbt(weapon));
    }
    if p.left_owner {
        put(&mut tag, "LeftOwner", Tag::Byte(1));
    }
    if let Some(state) = in_block {
        put(&mut tag, "inBlockState", state);
    }
    // Entity.saveWithoutId writes NoGravity only when set.
    if p.no_gravity {
        put(&mut tag, "NoGravity", Tag::Byte(1));
    }
    if let Some(owner) = owner {
        put(&mut tag, "Owner", Tag::IntArray(owner.to_vec()));
    }
    Some(tag)
}
/// A saved arrow as a live one (`readAdditionalSaveData`), with its
/// `inBlockState` compound for the caller to resolve. `effect` is the tipped
/// arrow's lasting effect from its item.
pub fn arrow_from_nbt(
    tag: &Tag,
    player: Option<[i32; 4]>,
    effect: impl FnOnce(&crate::items::ItemStack) -> Option<(&'static str, i32)>,
) -> Option<(Projectile, Option<Tag>)> {
    let position = position(tag)?;
    let owner = uuid_from_tag(tag.get("Owner"));
    let from_player = player.is_some() && owner == player;
    // `getDefaultPickupItem`: a `SpectralArrow` is picked up as its own item.
    let spectral = kind(tag).as_deref() == Some("minecraft:spectral_arrow");
    let default_item = if spectral {
        "minecraft:spectral_arrow"
    } else {
        "minecraft:arrow"
    };
    let item = tag
        .get("item")
        .and_then(item_from_nbt)
        .unwrap_or_else(|| crate::items::ItemStack::new(default_item, 1));
    let kind = ProjectileKind::Arrow {
        base_damage: float(tag.get("damage")).unwrap_or(2.0),
        critical: flag(tag, "crit"),
        effect: effect(&item),
    };
    let mut p = Projectile::new(kind, position, from_player.then_some(Who::Player));
    p.velocity = motion(tag);
    [p.yaw, p.pitch] = rotation(tag);
    p.left_owner = flag(tag, "LeftOwner");
    p.owner_uuid = owner;
    p.no_gravity = flag(tag, "NoGravity");
    let a = p.arrow.as_mut()?;
    a.life = number(tag.get("life")).unwrap_or(0) as i16 as i32;
    a.shake = (number(tag.get("shake")).unwrap_or(0) as i8 as u8).into();
    a.in_ground = flag(tag, "inGround");
    a.pickup = Pickup::from_byte(number(tag.get("pickup")).unwrap_or(0));
    a.pierce = number(tag.get("PierceLevel")).unwrap_or(0) as u8;
    a.has_been_shot = flag(tag, "HasBeenShot");
    a.fire_ticks = number(tag.get("Fire")).unwrap_or(0) as i16 as i32;
    a.fall_distance = float(tag.get("fall_distance")).unwrap_or(0.0);
    a.spectral = spectral;
    a.pickup_item = item;
    a.fired_from_weapon = tag.get("weapon").and_then(item_from_nbt);
    p.stuck = a.in_ground;
    Some((p, tag.get("inBlockState").cloned()))
}

/// `LargeFireball`, `SmallFireball` and `WitherSkull`: `AbstractHurtingProjectile`'s
/// `acceleration_power`, `Projectile`'s owner and flags, `ExplosionPower`,
/// and a skull's `dangerous`.
pub fn fireball_to_nbt(
    p: &Projectile,
    uuid: [i32; 4],
    owner: Option<[i32; 4]>,
    raw: Option<&Tag>,
) -> Option<Tag> {
    let power = match p.kind {
        ProjectileKind::LargeFireball { power } => Some(power),
        ProjectileKind::SmallFireball
        | ProjectileKind::WindCharge
        | ProjectileKind::BreezeWindCharge
        | ProjectileKind::WitherSkull { .. } => None,
        _ => return None,
    };
    let base = Base {
        id: p.kind.id(),
        uuid,
        position: p.position,
        motion: p.velocity,
        rotation: [p.yaw, p.pitch],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    put(
        &mut tag,
        "acceleration_power",
        Tag::Double(p.acceleration.to_bits()),
    );
    put_projectile(&mut tag, p, owner);
    // AbstractHurtingProjectile.tick: shouldBurn is true for both fireballs, so
    // once ticked they igniteForSeconds(1) every tick. WitherSkull.shouldBurn is
    // false, so a skull never saves a fire.
    if p.age > 0 && p.kind.is_item_fireball() {
        let fire = number(tag.get("Fire")).unwrap_or(0).max(20);
        put(&mut tag, "Fire", Tag::Short(fire as i16));
    }
    if let Some(power) = power {
        put(&mut tag, "ExplosionPower", Tag::Byte(power as i8));
    }
    // WitherSkull.addAdditionalSaveData: `dangerous`, false for a black skull.
    if let ProjectileKind::WitherSkull { dangerous } = p.kind {
        put(&mut tag, "dangerous", Tag::Byte(i8::from(dangerous)));
    }
    // Fireball.addAdditionalSaveData: the item `getItem` returns, a fire
    // charge unless `setItem` was given another (kept as a single one).
    if let Some(item) = &p.item {
        put(&mut tag, "Item", item_to_nbt(item));
    }
    Some(tag)
}

/// `Projectile.addAdditionalSaveData`: Owner, LeftOwner only when set, and
/// the explicit HasBeenShot flag.
fn put_projectile(tag: &mut Tag, p: &Projectile, owner: Option<[i32; 4]>) {
    if let Tag::Compound(map) = tag {
        map.remove(&"LeftOwner".into());
    }
    if p.left_owner {
        put(tag, "LeftOwner", Tag::Byte(1));
    }
    // Entity.saveWithoutId writes NoGravity only when set.
    if p.no_gravity {
        put(tag, "NoGravity", Tag::Byte(1));
    }
    put(tag, "HasBeenShot", Tag::Byte(i8::from(p.has_been_shot)));
    if let Some(owner) = owner {
        put(tag, "Owner", Tag::IntArray(owner.to_vec()));
    }
}

/// The thrown item projectiles Horizon saves: each entity type's id is also
/// the item it saves when none was loaded (`getDefaultItem`).
const THROWABLE_ITEMS: [&str; 6] = [
    "minecraft:snowball",
    "minecraft:egg",
    "minecraft:ender_pearl",
    "minecraft:splash_potion",
    "minecraft:lingering_potion",
    "minecraft:experience_bottle",
];
/// The item a throwable flies as: its stack (a potion's), the mod type's
/// item, or the item its entity type is named after.
fn throwable_item(kind: &ProjectileKind) -> Option<&str> {
    match kind {
        ProjectileKind::Thrown(t) => Some(t.item),
        ProjectileKind::Potion(stack, _) => Some(&stack.item),
        _ => THROWABLE_ITEMS.iter().find(|id| **id == kind.id()).copied(),
    }
}

/// The thrown kind an entity id names: vanilla's, or a content mod's type.
/// A potion flies as the item its entity type names, which is the item a
/// summoned one has when it names none (`getDefaultItem`).
pub fn throwable_kind(id: &str) -> Option<ProjectileKind> {
    if !THROWABLE_ITEMS.contains(&id) {
        return crate::projectiles::thrown_type(id).map(ProjectileKind::Thrown);
    }
    match id {
        "minecraft:snowball" => Some(ProjectileKind::Snowball),
        "minecraft:egg" => Some(ProjectileKind::Egg),
        "minecraft:ender_pearl" => Some(ProjectileKind::EnderPearl),
        // The entity type is the id; the `Item` read later may be another potion.
        "minecraft:experience_bottle" => Some(ProjectileKind::ExperienceBottle),
        _ => Some(ProjectileKind::Potion(
            Box::new(crate::items::ItemStack::new(id, 1)),
            id == "minecraft:lingering_potion",
        )),
    }
}

/// Whether its chunk saves it: the kinds `arrow_to_nbt`, `fireball_to_nbt`
/// and [`throwable_to_nbt`] write.
pub fn saved_with_chunk(p: &Projectile) -> bool {
    // No wildcard: a kind added to `ProjectileKind` must say here whether its
    // chunk saves it (`Entity.shouldBeSaved` is true for every projectile in
    // vanilla), and a `true` needs a writer in `entity_chunks`.
    match &p.kind {
        ProjectileKind::LargeFireball { .. }
        | ProjectileKind::SmallFireball
        | ProjectileKind::WindCharge
        | ProjectileKind::BreezeWindCharge
        | ProjectileKind::WitherSkull { .. }
        | ProjectileKind::Snowball
        | ProjectileKind::Egg
        | ProjectileKind::EnderPearl
        | ProjectileKind::Potion(..)
        | ProjectileKind::ExperienceBottle
        | ProjectileKind::LlamaSpit
        | ProjectileKind::ShulkerBullet { target: None, .. }
        | ProjectileKind::Thrown(_) => true,
        ProjectileKind::Arrow { .. } => p.arrow.is_some(),
        // Not written yet: tridents save from their own list, and a bullet
        // with a target would lose it.
        ProjectileKind::Trident
        | ProjectileKind::ShulkerBullet {
            target: Some(_), ..
        } => false,
    }
}

/// `ThrowableItemProjectile.addAdditionalSaveData`: `Projectile`'s keys and
/// the `Item` it flies as.
pub fn throwable_to_nbt(
    p: &Projectile,
    uuid: [i32; 4],
    owner: Option<[i32; 4]>,
    raw: Option<&Tag>,
) -> Option<Tag> {
    let item = throwable_item(&p.kind)?;
    let base = Base {
        id: p.kind.id(),
        uuid,
        position: p.position,
        motion: p.velocity,
        rotation: [p.yaw, p.pitch],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    put_projectile(&mut tag, p, owner);
    // Entity.saveWithoutId writes NoGravity only when set.
    if p.no_gravity {
        put(&mut tag, "NoGravity", Tag::Byte(1));
    }
    // the loaded item stays (a blue egg's, a renamed one's, a potion's
    // effects); when there is none, `getItem`'s default is written.
    if tag.get("Item").is_none() {
        let stack = match &p.kind {
            ProjectileKind::Potion(stack, _) => (**stack).clone(),
            _ => crate::items::ItemStack::new(item, 1),
        };
        put(&mut tag, "Item", item_to_nbt(&stack));
    }
    Some(tag)
}

/// `ThrowableItemProjectile.readAdditionalSaveData` for the kinds
/// [`throwable_to_nbt`] writes; the owner is the player when it is theirs.
pub fn throwable_from_nbt(tag: &Tag, player: Option<[i32; 4]>) -> Option<Projectile> {
    let mut kind = throwable_kind(&kind(tag)?)?;
    // `readAdditionalSaveData` sets the `Item` it flies as, potion effects
    // and all; a tag without one keeps `getItem`'s default.
    if let ProjectileKind::Potion(stack, _) = &mut kind
        && let Some(item) = tag.get("Item").and_then(item_from_nbt)
    {
        **stack = item;
    }
    let owner = owner_uuid(tag);
    let from_player = player.is_some() && owner == player;
    let mut p = Projectile::new(kind, position(tag)?, from_player.then_some(Who::Player));
    p.velocity = motion(tag);
    [p.yaw, p.pitch] = rotation(tag);
    p.left_owner = flag(tag, "LeftOwner");
    p.has_been_shot = flag(tag, "HasBeenShot");
    p.owner_uuid = owner;
    p.no_gravity = flag(tag, "NoGravity");
    Some(p)
}

/// `LlamaSpit` and `ShulkerBullet` as `/summon` or a chunk loads them:
/// `Projectile`'s `Owner`, `LeftOwner`, `NoGravity` and the motion. A bullet
/// loaded without a `Target` has none, so it falls (`ShulkerBullet.tick`).
pub fn spit_or_bullet_from_nbt(tag: &Tag, player: Option<[i32; 4]>) -> Option<Projectile> {
    let kind = match kind(tag)?.as_str() {
        "minecraft:llama_spit" => ProjectileKind::LlamaSpit,
        "minecraft:shulker_bullet" => ProjectileKind::ShulkerBullet {
            target: None,
            dir: None,
            steps: 0,
            delta: [0.0; 3],
            seed: 0,
        },
        _ => return None,
    };
    let owner = owner_uuid(tag);
    let from_player = player.is_some() && owner == player;
    let mut p = Projectile::new(kind, position(tag)?, from_player.then_some(Who::Player));
    p.velocity = motion(tag);
    [p.yaw, p.pitch] = rotation(tag);
    p.left_owner = flag(tag, "LeftOwner");
    p.has_been_shot = flag(tag, "HasBeenShot");
    p.owner_uuid = owner;
    p.no_gravity = flag(tag, "NoGravity");
    Some(p)
}

/// The save of [`spit_or_bullet_from_nbt`]'s kinds: `Projectile`'s keys.
pub fn spit_or_bullet_to_nbt(
    p: &Projectile,
    uuid: [i32; 4],
    owner: Option<[i32; 4]>,
    raw: Option<&Tag>,
) -> Option<Tag> {
    if !matches!(
        p.kind,
        ProjectileKind::LlamaSpit | ProjectileKind::ShulkerBullet { target: None, .. }
    ) {
        return None;
    }
    let base = Base {
        id: p.kind.id(),
        uuid,
        position: p.position,
        motion: p.velocity,
        rotation: [p.yaw, p.pitch],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    put_projectile(&mut tag, p, owner);
    Some(tag)
}

/// `Projectile`'s `Owner`.
pub fn owner_uuid(tag: &Tag) -> Option<[i32; 4]> {
    uuid_from_tag(tag.get("Owner"))
}

/// `Fireball.readAdditionalSaveData` (and, around it, `Projectile`'s Owner and
/// LeftOwner and `AbstractHurtingProjectile`'s acceleration power): a fireball
/// also reads the item it is drawn as through `setItem`, whose default is the
/// fire charge; a wind charge is no `Fireball` and has none.
pub fn fireball_from_nbt(tag: &Tag, player: Option<[i32; 4]>) -> Option<Projectile> {
    let kind = match kind(tag)?.as_str() {
        "minecraft:fireball" => ProjectileKind::LargeFireball {
            power: number(tag.get("ExplosionPower")).unwrap_or(1) as i32,
        },
        "minecraft:small_fireball" => ProjectileKind::SmallFireball,
        "minecraft:wind_charge" => ProjectileKind::WindCharge,
        "minecraft:breeze_wind_charge" => ProjectileKind::BreezeWindCharge,
        // WitherSkull.readAdditionalSaveData: an absent `dangerous` is a black skull.
        "minecraft:wither_skull" => ProjectileKind::WitherSkull {
            dangerous: flag(tag, "dangerous"),
        },
        _ => return None,
    };
    let owner = uuid_from_tag(tag.get("Owner"));
    let from_player = player.is_some() && owner == player;
    let mut p = Projectile::new(kind, position(tag)?, from_player.then_some(Who::Player));
    p.velocity = motion(tag);
    [p.yaw, p.pitch] = rotation(tag);
    p.left_owner = flag(tag, "LeftOwner");
    p.has_been_shot = flag(tag, "HasBeenShot");
    p.owner_uuid = owner;
    p.no_gravity = flag(tag, "NoGravity");
    p.acceleration = float(tag.get("acceleration_power")).unwrap_or(0.1);
    if let Some(item) = tag.get("Item").and_then(item_from_nbt) {
        p.set_item(item);
    }
    Some(p)
}

/// `FireworkRocketEntity.readAdditionalSaveData`: Life, LifeTime (0 when
/// absent: a bare `/summon` explodes at once), FireworksItem and
/// ShotAtAngle, with `Projectile`'s Owner and LeftOwner.
pub struct RocketNbt {
    pub position: Vec3,
    pub motion: Vec3,
    pub life: i32,
    pub lifetime: i32,
    pub item: Option<crate::items::ItemStack>,
    pub angled: bool,
    pub owner: Option<Who>,
    pub left_owner: bool,
}
pub fn firework_rocket_from_nbt(tag: &Tag, player: Option<[i32; 4]>) -> Option<RocketNbt> {
    let owner = uuid_from_tag(tag.get("Owner"));
    Some(RocketNbt {
        position: position(tag)?,
        motion: motion(tag),
        life: number(tag.get("Life")).unwrap_or(0) as i32,
        lifetime: number(tag.get("LifeTime")).unwrap_or(0) as i32,
        item: tag.get("FireworksItem").and_then(item_from_nbt),
        angled: flag(tag, "ShotAtAngle"),
        owner: (player.is_some() && owner == player).then_some(Who::Player),
        left_owner: flag(tag, "LeftOwner"),
    })
}

/// What `FireworkRocketEntity.addAdditionalSaveData` writes, over `Projectile`'s
/// Owner and LeftOwner.
pub struct RocketSave<'a> {
    pub rotation: [f32; 2],
    pub position: Vec3,
    pub motion: Vec3,
    pub life: i32,
    pub lifetime: i32,
    pub item: &'a crate::items::ItemStack,
    pub angled: bool,
    pub owner: Option<[i32; 4]>,
    pub left_owner: bool,
}
pub fn firework_rocket_to_nbt(r: &RocketSave, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let mut tag = Base {
        id: "minecraft:firework_rocket",
        uuid,
        position: r.position,
        motion: r.motion,
        rotation: r.rotation,
        on_ground: false,
        fall_distance: 0.0,
    }
    .tag(raw);
    for (key, value) in [
        ("Life", Tag::Int(r.life)),
        ("LifeTime", Tag::Int(r.lifetime)),
        ("FireworksItem", item_to_nbt(r.item)),
        ("ShotAtAngle", Tag::Byte(i8::from(r.angled))),
        ("HasBeenShot", Tag::Byte(1)),
    ] {
        put(&mut tag, key, value);
    }
    if r.left_owner {
        put(&mut tag, "LeftOwner", Tag::Byte(1));
    }
    if let Some(owner) = r.owner {
        put(&mut tag, "Owner", Tag::IntArray(owner.to_vec()));
    }
    tag
}

/// A thrown trident as `AbstractArrow` + `ThrownTrident` save it.
pub struct TridentSave<'a> {
    pub position: Vec3,
    pub motion: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub in_ground: bool,
    pub life: i32,
    pub shake: u32,
    pub dealt_damage: bool,
    pub creative_only: bool,
    pub no_gravity: bool,
    pub fire_ticks: i32,
    pub item: &'a crate::items::ItemStack,
}
pub fn trident_to_nbt(t: &TridentSave, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let mut tag = Base {
        id: "minecraft:trident",
        uuid,
        position: t.position,
        motion: t.motion,
        rotation: [t.yaw, t.pitch],
        on_ground: false,
        fall_distance: 0.0,
    }
    .tag(raw);
    // Projectile.addAdditionalSaveData: LeftOwner once set. A trident keeps no
    // owner, and an ownerless projectile has left it on its first tick.
    if let Tag::Compound(map) = &mut tag {
        map.entry("LeftOwner".into()).or_insert(Tag::Byte(1));
    }
    for (key, value) in [
        ("life", Tag::Short(t.life as i16)),
        ("shake", Tag::Byte(t.shake as i8)),
        ("inGround", Tag::Byte(i8::from(t.in_ground))),
        // AbstractArrow.Pickup: 0 disallowed, 1 allowed, 2 creative only.
        ("pickup", Tag::Byte(if t.creative_only { 2 } else { 1 })),
        ("damage", Tag::Double(2.0_f64.to_bits())),
        ("HasBeenShot", Tag::Byte(1)),
        ("crit", Tag::Byte(0)),
        ("PierceLevel", Tag::Byte(0)),
        ("SoundEvent", string("minecraft:item.trident.hit_ground")),
        ("item", item_to_nbt(t.item)),
        ("DealtDamage", Tag::Byte(i8::from(t.dealt_damage))),
        ("Fire", Tag::Short(t.fire_ticks as i16)),
    ] {
        put(&mut tag, key, value);
    }
    // Entity.saveWithoutId writes NoGravity only when set.
    if t.no_gravity {
        put(&mut tag, "NoGravity", Tag::Byte(1));
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"NoGravity".into());
    }
    tag
}
/// (position, motion, yaw, pitch, in ground, life, shake, dealt damage,
/// creative only, the trident item)
pub struct TridentNbt {
    pub position: Vec3,
    pub motion: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub in_ground: bool,
    pub life: i32,
    pub shake: u32,
    pub dealt_damage: bool,
    pub creative_only: bool,
    pub no_gravity: bool,
    pub fire_ticks: i32,
    pub item: crate::items::ItemStack,
}
pub fn trident_from_nbt(tag: &Tag) -> Option<TridentNbt> {
    let [yaw, pitch] = rotation(tag);
    Some(TridentNbt {
        position: position(tag)?,
        motion: motion(tag),
        yaw,
        pitch,
        in_ground: flag(tag, "inGround"),
        life: number(tag.get("life")).unwrap_or(0) as i32,
        shake: number(tag.get("shake")).unwrap_or(0).max(0) as u32,
        dealt_damage: flag(tag, "DealtDamage"),
        creative_only: number(tag.get("pickup")) == Some(2),
        no_gravity: flag(tag, "NoGravity"),
        fire_ticks: number(tag.get("Fire")).unwrap_or(0) as i32,
        item: tag
            .get("item")
            .and_then(item_from_nbt)
            .unwrap_or_else(|| crate::items::ItemStack::new("minecraft:trident", 1)),
    })
}

/// `FallingBlockEntity`: BlockState, Time, DropItem, HurtEntities.
#[allow(clippy::too_many_arguments)]
pub fn falling_block_to_nbt(
    state: Tag,
    position: Vec3,
    motion: Vec3,
    time: i32,
    drop_item: bool,
    damage: crate::falling_block::Damage,
    fall_distance: f64,
    rotation: [f32; 2],
    uuid: [i32; 4],
) -> Tag {
    let base = Base {
        id: "minecraft:falling_block",
        uuid,
        position,
        motion,
        rotation,
        on_ground: false,
        fall_distance,
    };
    let mut tag = base.tag(None);
    for (key, value) in [
        ("BlockState", state),
        ("Time", Tag::Int(time)),
        ("DropItem", Tag::Byte(i8::from(drop_item))),
    ] {
        put(&mut tag, key, value);
    }
    damage.save(&mut tag);
    tag
}
/// (block state tag, position, motion, time, fall distance)
pub fn falling_block_from_nbt(tag: &Tag) -> Option<(Tag, Vec3, Vec3, i32, f64)> {
    Some((
        tag.get("BlockState")?.clone(),
        position(tag)?,
        motion(tag),
        number(tag.get("Time")).unwrap_or(0) as i32,
        float(tag.get("fall_distance")).unwrap_or(0.0),
    ))
}

/// `NoGravity` and `DropItem` (true when absent) of a saved falling block.
pub fn falling_block_flags(tag: &Tag) -> (bool, bool) {
    (
        flag(tag, "NoGravity"),
        number(tag.get("DropItem")).is_none_or(|v| v != 0),
    )
}

/// Painting.addAdditionalSaveData: `facing`, `variant` and `block_pos`.
pub fn painting_to_nbt(p: &Painting, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let (lo, hi) = p.bounds();
    let base = Base {
        id: "minecraft:painting",
        uuid,
        position: Vec3::new(
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ),
        motion: Vec3::ZERO,
        rotation: [f32::from(p.facing) * 90.0, 0.0],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    put(&mut tag, "facing", Tag::Byte(p.facing as i8));
    put(&mut tag, "variant", string(&p.variant));
    put(&mut tag, "block_pos", Tag::IntArray(p.pos.to_vec()));
    tag
}
/// `BlockAttachedEntity.readAdditionalSaveData`: `block_pos` when saved; a
/// summoned one has none, and `setPos` puts its block at the block holding
/// the summon position.
fn attached_block_pos(tag: &Tag) -> Option<[i32; 3]> {
    match tag.get("block_pos") {
        Some(Tag::IntArray(v)) if v.len() == 3 => Some([v[0], v[1], v[2]]),
        _ => {
            let at = position(tag)?;
            Some([
                at.x.floor() as i32,
                at.y.floor() as i32,
                at.z.floor() as i32,
            ])
        }
    }
}
/// The block position, facing and variant of a saved painting.
pub fn painting_from_nbt(tag: &Tag) -> Option<([i32; 3], u8, String)> {
    let pos = attached_block_pos(tag)?;
    let facing = (number(tag.get("facing")).unwrap_or(0) & 3) as u8;
    Some((pos, facing, text(tag.get("variant"))?))
}
/// Cushion.addAdditionalSaveData: the block it sits on and its colour.
pub fn cushion_to_nbt(c: &Cushion, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let base = Base {
        id: "minecraft:cushion",
        uuid,
        position: c.position,
        motion: Vec3::ZERO,
        rotation: [c.yaw, 0.0],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    put(&mut tag, "block_pos", Tag::IntArray(c.pos.to_vec()));
    put(&mut tag, "color", string(c.color_name()));
    if let Some(name) = &c.custom_name {
        put(&mut tag, "CustomName", name.clone());
    }
    tag
}
/// The position, block position, yaw and colour of a saved cushion.
pub fn cushion_from_nbt(tag: &Tag, id: u32) -> Option<Cushion> {
    let position = position(tag)?;
    let pos = match tag.get("block_pos") {
        Some(Tag::IntArray(v)) if v.len() == 3 => [v[0], v[1], v[2]],
        _ => [
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        ],
    };
    let yaw = match tag.get("Rotation") {
        Some(Tag::List(l)) => l
            .first()
            .and_then(|v| crate::save::float(Some(v)))
            .unwrap_or(0.0) as f32,
        _ => 0.0,
    };
    let color =
        crate::entities::dye_index(&text(tag.get("color")).unwrap_or_else(|| "white".into()));
    Some(Cushion {
        attachment_lifetime: Default::default(),
        join: Default::default(),
        id,
        position,
        pos,
        yaw,
        color,
        check_interval: 0,
        custom_name: tag.get("CustomName").cloned(),
    })
}

#[cfg(test)]
mod cushion_tests {
    use super::*;

    #[test]
    fn cushion_save_preserves_color_anchor_rotation_and_unknown_data() {
        let cushion = Cushion {
            attachment_lifetime: Default::default(),
            join: Default::default(),
            id: 7,
            position: Vec3::new(-1.5, 64.5, 3.5),
            pos: [-2, 64, 3],
            yaw: 270.0,
            color: 14,
            check_interval: 55,
            custom_name: None,
        };
        let raw = Tag::compound([("CustomName", string("Seat"))]);
        let tag = cushion_to_nbt(&cushion, [1, 2, 3, 4], Some(&raw));
        assert_eq!(text(tag.get("CustomName")).as_deref(), Some("Seat"));
        let loaded = cushion_from_nbt(&tag, 8).unwrap();
        assert_eq!(loaded.id, 8);
        assert_eq!(loaded.position, cushion.position);
        assert_eq!(loaded.pos, cushion.pos);
        assert_eq!(loaded.yaw, cushion.yaw);
        assert_eq!(loaded.color_name(), "red");
        assert_eq!(loaded.check_interval, 0);
        assert_eq!(loaded.bounds(), cushion.bounds());
    }
}
/// ItemFrame.addAdditionalSaveData.
pub fn item_frame_to_nbt(f: &ItemFrame, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let (lo, hi) = f.bounds();
    let base = Base {
        id: f.kind(),
        uuid,
        position: Vec3::new(
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ),
        motion: Vec3::ZERO,
        rotation: [0.0, 0.0],
        on_ground: false,
        fall_distance: 0.0,
    };
    let mut tag = base.tag(raw);
    // `Entity.saveWithoutId`: `CustomName` before the frame's own fields.
    if let Some(name) = &f.custom_name {
        put(&mut tag, "CustomName", name.clone());
    } else if let Tag::Compound(map) = &mut tag {
        map.remove(&"CustomName".into());
    }
    put(&mut tag, "Facing", Tag::Byte(f.facing as i8));
    put(&mut tag, "block_pos", Tag::IntArray(f.pos.to_vec()));
    put(&mut tag, "ItemRotation", Tag::Byte(f.rotation as i8));
    put(
        &mut tag,
        "ItemDropChance",
        Tag::Float(f.drop_chance.to_bits()),
    );
    put(&mut tag, "Invisible", Tag::Byte(i8::from(f.invisible)));
    put(&mut tag, "Fixed", Tag::Byte(i8::from(f.fixed)));
    if let Tag::Compound(map) = &mut tag {
        map.remove(&horizon_core::nbt::JavaString::from("Item"));
    }
    if let Some(item) = &f.item {
        put(&mut tag, "Item", item_to_nbt(item));
    }
    tag
}
/// `ItemFrame.readAdditionalSaveData`: an absent Facing defaults to Direction.DOWN.
pub fn item_frame_from_nbt(tag: &Tag, id: u32) -> Option<ItemFrame> {
    let pos = attached_block_pos(tag)?;
    Some(ItemFrame {
        attachment_lifetime: Default::default(),
        join: Default::default(),
        id,
        pos,
        facing: number(tag.get("Facing")).unwrap_or(0).clamp(0, 5) as u8,
        glow: kind(tag).as_deref() == Some("minecraft:glow_item_frame"),
        item: tag.get("Item").and_then(item_from_nbt),
        rotation: (number(tag.get("ItemRotation")).unwrap_or(0) & 7) as u8,
        drop_chance: float(tag.get("ItemDropChance")).map_or(1.0, |v| v as f32),
        fixed: flag(tag, "Fixed"),
        invisible: flag(tag, "Invisible"),
        check_interval: 0,
        custom_name: tag.get("CustomName").cloned(),
    })
}
/// ArmorStand.addAdditionalSaveData with LivingEntity's `equipment`.
pub fn armor_stand_to_nbt(a: &ArmorStand, uuid: [i32; 4], raw: Option<&Tag>) -> Tag {
    let base = Base {
        id: "minecraft:armor_stand",
        uuid,
        position: a.position,
        motion: a.velocity,
        rotation: [a.yaw, a.pitch],
        on_ground: a.on_ground,
        fall_distance: a.enchantment_fall_distance,
    };
    let mut tag = base.non_mob_tag(raw);
