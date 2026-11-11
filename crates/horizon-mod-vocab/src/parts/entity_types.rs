use super::*;

/// The design's provisional bounds for general entity types (#792), declared
/// once so the registrar, the codecs and the tests quote the same numbers.
/// The payload bound applies to mod-owned payload bytes only, never to a
/// record of a vanilla type. The quarantine bounds (provisional, no vanilla
/// analogue) cap what stage 3 preserves of a missing mod's records.
// exhaustive: the design's limits are one block; no further constants join it
pub mod limits {
    /// The most payload bytes one general entity record may carry. A payload
    /// one byte larger is refused, never truncated into a valid-looking
    /// entity.
    pub const PAYLOAD_LIMIT: usize = 65_536;
    /// The deepest nesting of a general entity's decoded data. Stage 3's
    /// payload decode checks it; the record tree itself has the stricter
    /// passenger-depth bound.
    pub const MAX_NESTING: u32 = 32;
    /// The deepest chain of passengers a general entity tree may carry.
    pub const MAX_PASSENGER_DEPTH: u32 = 16;
    /// The largest finite positive dimension (width or height, in blocks) a
    /// declared type may name.
    pub const MAX_DIMENSION: f64 = 64.0;
    /// The most chunks of tracking distance a declared type may name.
    pub const MAX_TRACKING_CHUNKS: u8 = 32;
    /// The update interval a declared type may name, in ticks, inclusive.
    pub const UPDATE_INTERVAL_MIN: u16 = 1;
    /// See [`UPDATE_INTERVAL_MIN`].
    pub const UPDATE_INTERVAL_MAX: u16 = 1200;
    /// The largest record a missing mod's quarantined general entity may
    /// occupy, as its encoded NBT size. A larger record is skipped with a
    /// warning rather than preserved. Provisional, no vanilla analogue:
    /// vanilla deletes unknown-mod entities outright, and Horizon's quarantine
    /// is a #792 stage-3 choice pending Ari's decision on the issue.
    pub const QUARANTINE_RECORD_LIMIT: u64 = 69_632;
    /// The most quarantined general entity records one chunk may carry. The
    /// next one is skipped with a warning. Provisional, no vanilla analogue:
    /// vanilla has no quarantine to bound.
    pub const QUARANTINE_CHUNK_LIMIT: usize = 256;
}

/// A general entity type's spawn category (#792's design supplies category at
/// declaration; general types start as `MISC`, which never gains natural
/// spawning implicitly).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum GeneralEntityCategory {
    /// Neither monster nor creature: no natural-spawn policy of its own.
    Misc,
}

/// Whether instances of a declared general type persist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum GeneralSavePolicy {
    /// Instances save like ordinary entities do.
    Save,
    /// `noSave`: instances produce zero persisted records.
    NoSave,
}

/// A general entity type a mod declares, as the game reads it when it freezes
/// the type registry (#792). The registry is data: no mod type adds an entry
/// to any hand-kept engine list.
#[derive(Clone, Debug)]
// exhaustive: only registration builds it; the game reads it
#[derive(horizon_mod_api_macros::Wire)]
pub struct GeneralEntityTypeDeclaration {
    /// The declaring mod's id: the first key of the registry's sort order.
    pub mod_id: String,
    /// The type's namespaced id (`mymod:robot`).
    pub id: String,
    /// The type's spawn category.
    pub category: GeneralEntityCategory,
    /// The type's finite positive width in blocks, at most
    /// [`limits::MAX_DIMENSION`].
    pub width: f64,
    /// The type's finite positive height in blocks, at most
    /// [`limits::MAX_DIMENSION`].
    pub height: f64,
    /// Whether instances of the type persist.
    pub save_policy: GeneralSavePolicy,
    /// Whether the type may be summoned by name.
    pub summonable: bool,
    /// The tracking distance in chunks, at most [`limits::MAX_TRACKING_CHUNKS`].
    pub tracking_chunks: u8,
    /// The tracking update interval in ticks, inside
    /// [`limits::UPDATE_INTERVAL_MIN`]`..=`[`limits::UPDATE_INTERVAL_MAX`].
    pub update_interval: u16,
    /// The payload schema version instances of the type carry.
    pub payload_version: u16,
    /// The ids of loaded mods this type requires, every one of which must be
    /// present for the declaration to stand.
    pub required_features: Vec<String>,
}

/// Why a mod's general entity type declaration was refused
/// ([`Registrar::declare_general_entity`](GeneralTypesRegistrar::declare_general_entity)).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum GeneralEntityRegistrationError {
    /// The registry is frozen: declarations are only accepted while the game
    /// is still in its registration phase, before any world or connection
    /// opens.
    Frozen,
    /// Only both-side packages may declare entity types, which the network
    /// names.
    WrongSide,
    /// The type's id is malformed, empty or in a namespace the declaring mod
    /// does not own (a vanilla type's included).
    InvalidId,
    /// Another declaration already covers the type.
    Duplicate,
    /// A dimension is NaN, infinite, zero or negative.
    NotFinite,
    /// A dimension exceeds [`limits::MAX_DIMENSION`], the tracking distance
    /// exceeds [`limits::MAX_TRACKING_CHUNKS`], or the update interval is
    /// outside its bounds.
    InvalidBounds,
    /// A required feature names no loaded mod.
    UnknownDependency,
}

/// The general entity types mods declared.
pub fn general_entity_type_declarations() -> &'static [GeneralEntityTypeDeclaration] {
    installed().map_or(&[], |installed| &installed.general_entity_types)
}

/// Registration of general entity types (#792): one frozen registry of
/// namespaced types, filled before worlds or connections open.
pub trait GeneralTypesRegistrar {
    /// Declare a general entity type of the mod's own (vanilla's
    /// `EntityType` builder, NeoForge's `RegisterEvent`): `path` in the mod's
    /// namespace (`robot` of `mymod` is `mymod:robot`), its category, its
    /// finite positive `width` and `height` in blocks, its save and summon
    /// policy, its bounded tracking distance and update interval, its payload
    /// schema version and the loaded mods it requires.
    ///
    /// An id outside the mod's manifest-owned namespaces, a malformed id, a
    /// duplicate of this mod's or another's, a NaN or non-positive dimension,
    /// a bound the design's limits refuse, a required feature naming no
    /// loaded mod, or a declaration after the registry froze (the
    /// registration phase has ended) refuses, naming the mod, without
    /// panicking. Bad or unbounded input is always refused, never clamped.
    fn declare_general_entity(
        &mut self,
        path: &str,
        category: GeneralEntityCategory,
        width: f64,
        height: f64,
        save_policy: GeneralSavePolicy,
        summonable: bool,
        tracking_chunks: u8,
        update_interval: u16,
        payload_version: u16,
        required_features: &[&str],
    ) -> Result<(), GeneralEntityRegistrationError>;
}

impl GeneralTypesRegistrar for PartRegistrar {
    fn declare_general_entity(
        &mut self,
        path: &str,
        category: GeneralEntityCategory,
        width: f64,
        height: f64,
        save_policy: GeneralSavePolicy,
        summonable: bool,
        tracking_chunks: u8,
        update_interval: u16,
        payload_version: u16,
        required_features: &[&str],
    ) -> Result<(), GeneralEntityRegistrationError> {
        use GeneralEntityRegistrationError as Error;
        // The registry freezes with the phase: once registration ends and any
        // later phase begins, no declaration is accepted anywhere.
        if crate::world::phase::current_phase() != crate::world::phase::ModPhase::Registration {
            self.refused.push(format!(
                "mod {} declares a general entity type after the registry froze",
                self.mod_id
            ));
            return Err(Error::Frozen);
        }
        let id = match self.owned_declaration_id(path) {
            Ok(id) => id,
            Err(reason) => {
                self.refused.push(reason);
                return Err(Error::InvalidId);
            }
        };
        let id_for_log = id.clone();
        let result = (|| {
            if crate::LoadedMods::new()
                .get(self.mod_id)
                .is_some_and(|package| package.side() != crate::Side::Both)
            {
                return Err(Error::WrongSide);
            }
            if self
                .general_entity_types
                .iter()
                .any(|declared| declared.id == id)
            {
                return Err(Error::Duplicate);
            }
            if ![width, height].into_iter().all(f64::is_finite) || width <= 0.0 || height <= 0.0 {
                return Err(Error::NotFinite);
            }
            if width > limits::MAX_DIMENSION
                || height > limits::MAX_DIMENSION
                || tracking_chunks > limits::MAX_TRACKING_CHUNKS
                || !(limits::UPDATE_INTERVAL_MIN..=limits::UPDATE_INTERVAL_MAX)
                    .contains(&update_interval)
            {
                return Err(Error::InvalidBounds);
            }
            for feature in required_features {
                if crate::LoadedMods::new().get(feature).is_none() {
                    return Err(Error::UnknownDependency);
                }
            }
            self.general_entity_types
                .push(GeneralEntityTypeDeclaration {
                    mod_id: self.mod_id.to_owned(),
                    id,
                    category,
                    width,
                    height,
                    save_policy,
                    summonable,
                    tracking_chunks,
                    update_interval,
                    payload_version,
                    required_features: required_features.iter().map(|&f| f.to_owned()).collect(),
                });
            Ok(())
        })();
        if let Err(error) = &result {
            self.refused.push(format!(
                "mod {} refuses general entity type {id_for_log:?}: {error:?}",
                self.mod_id
            ));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GeneralEntityRegistrationError as Error;
    use crate::world::phase::{ModPhase, PhaseOwner, PhaseScope};

    fn registrar() -> (PhaseScope, PartRegistrar) {
        // Declarations are phase-owned; tests scope their thread to the
        // registration phase the way the loader does, restoring it after.
        let scope = PhaseOwner::scope(ModPhase::Registration);
        (scope, PartRegistrar::new("mymod"))
    }

    fn declare(
        registrar: &mut PartRegistrar,
        path: &str,
        width: f64,
        height: f64,
        tracking_chunks: u8,
        update_interval: u16,
        required: &[&str],
    ) -> Result<(), Error> {
        registrar.declare_general_entity(
            path,
            GeneralEntityCategory::Misc,
            width,
            height,
            GeneralSavePolicy::Save,
            true,
            tracking_chunks,
            update_interval,
            1,
            required,
        )
    }

    /// The registration's checks: an id outside the mod's namespace (a
    /// vanilla type's included), a duplicate, non-finite or non-positive
    /// dimensions, bounds the design's limits refuse and a required feature
    /// naming no loaded mod all refuse, naming the mod; the good declaration
    /// stands beside them.
    #[test]
    pub(crate) fn general_entity_type_declarations_validate() {
        let (_phase, mut registrar) = registrar();
        declare(&mut registrar, "robot", 0.8, 1.9, 8, 3, &[]).unwrap();
        // The path stays in the declaring mod's namespace.
        assert_eq!(
            declare(&mut registrar, "minecraft:creeper", 0.6, 1.7, 8, 3, &[]),
            Err(Error::InvalidId)
        );
        assert_eq!(
            declare(&mut registrar, "foreign:robot", 0.6, 1.7, 8, 3, &[]),
            Err(Error::InvalidId)
        );
        assert_eq!(
            declare(&mut registrar, "Not An Id", 0.6, 1.7, 8, 3, &[]),
            Err(Error::InvalidId)
        );
        // The same type twice, whatever declared it, is a duplicate.
        assert_eq!(
            declare(&mut registrar, "robot", 0.5, 1.0, 4, 2, &[]),
            Err(Error::Duplicate)
        );
        // NaN, infinite, zero and negative dimensions.
        assert_eq!(
            declare(&mut registrar, "wide", f64::NAN, 1.0, 8, 3, &[]),
            Err(Error::NotFinite)
        );
        assert_eq!(
            declare(&mut registrar, "wide", f64::INFINITY, 1.0, 8, 3, &[]),
            Err(Error::NotFinite)
        );
        assert_eq!(
            declare(&mut registrar, "flat", 0.0, 1.0, 8, 3, &[]),
            Err(Error::NotFinite)
        );
        assert_eq!(
            declare(&mut registrar, "dark", -0.1, 1.0, 8, 3, &[]),
            Err(Error::NotFinite)
        );
        // The design's bounds: dimensions, tracking distance, interval.
        assert_eq!(
            declare(&mut registrar, "titan", 64.001, 1.0, 8, 3, &[]),
            Err(Error::InvalidBounds)
        );
        assert_eq!(
            declare(&mut registrar, "tower", 1.0, 65.0, 8, 3, &[]),
            Err(Error::InvalidBounds)
        );
        assert_eq!(
            declare(&mut registrar, "far", 1.0, 1.0, 33, 3, &[]),
            Err(Error::InvalidBounds)
        );
        assert_eq!(
            declare(&mut registrar, "slow", 1.0, 1.0, 8, 0, &[]),
            Err(Error::InvalidBounds)
        );
        assert_eq!(
            declare(&mut registrar, "slower", 1.0, 1.0, 8, 1201, &[]),
            Err(Error::InvalidBounds)
        );
        // A required feature naming no loaded mod.
        assert_eq!(
            declare(&mut registrar, "needy", 1.0, 1.0, 8, 3, &["missing:mod"]),
            Err(Error::UnknownDependency)
        );
        // Every refusal was reported, and the good declaration stands.
        assert_eq!(registrar.general_entity_types.len(), 1);
        assert_eq!(registrar.general_entity_types[0].id, "mymod:robot");
        assert_eq!(registrar.refused.len(), 14);
    }

    /// The registry is frozen outside the registration phase: a declaration
    /// then refuses with `Frozen`, whatever it names, and the frozen table is
    /// unchanged.
    #[test]
    pub(crate) fn general_entity_types_frozen_outside_registration() {
        // Scoped, because the thread pool reuses threads across tests: this
        // test must see `World` even after another test's registration scope.
        let _world = PhaseOwner::scope(ModPhase::World);
        let mut registrar = PartRegistrar::new("mymod");
        assert_eq!(
            declare(&mut registrar, "robot", 0.8, 1.9, 8, 3, &[]),
            Err(Error::Frozen)
        );
        assert!(registrar.general_entity_types.is_empty());
        assert_eq!(registrar.refused.len(), 1);
    }

    /// A declaration from a mod the test inventory does not list stands: the
    /// side check refuses only a listed non-both-side package, exactly as the
    /// attribute registration's does.
    #[test]
    pub(crate) fn general_entity_types_unlisted_mod_declares() {
        let (_phase, mut registrar) = registrar();
        declare(&mut registrar, "robot", 0.8, 1.9, 8, 3, &[]).unwrap();
        assert_eq!(registrar.general_entity_types.len(), 1);
        assert!(registrar.refused.is_empty());
    }

    /// A mod's `Hooks::register` receives `&mut dyn Registrar`, and the
    /// generated `Registrar` trait carries `GeneralTypesRegistrar` among its
    /// supertraits: the declaration reaches mods through the registrar they
    /// already hold, with no engine type leaking. Red if the supertrait
    /// wiring is lost: `declare_general_entity` would not resolve on
    /// `dyn Registrar`.
    /// Two mods may own one namespace (a manifest's `namespaces`), but their
    /// registrations run against the one registrar `register_all` drives
    /// (`parts/shared.rs`), so the second mod's declaration of the same key
    /// is a `Duplicate` at registration, with a refusal naming it — never a
    /// panic in the game thread at freeze. Red without the duplicate check:
    /// the second declaration would stand and the frozen registry would hit
    /// the unreachable.
    #[test]
    pub(crate) fn general_entity_types_two_mods_one_key_refuse_the_second() {
        use crate::world::phase::{
            PackageInfo, PackageKind, with_test_package_inventory,
        };
        let packages = vec![
            PackageInfo::new(
                "amod".to_owned(),
                "1".to_owned(),
                PackageKind::Code,
                crate::Side::Both,
                vec!["shared".to_owned()],
                vec![],
            ),
            PackageInfo::new(
                "bmod".to_owned(),
                "1".to_owned(),
                PackageKind::Code,
                crate::Side::Both,
                vec!["shared".to_owned()],
                vec![],
            ),
        ];
        let _scope = PhaseOwner::scope(ModPhase::Registration);
        with_test_package_inventory(packages, || {
            let mut registrar = PartRegistrar::new("amod");
            declare(&mut registrar, "shared:x", 0.8, 1.9, 8, 3, &[]).unwrap();
            // What `register_all` does between mods: the same registrar, the
            // next mod's id.
            registrar.mod_id = "bmod";
            assert_eq!(
                declare(&mut registrar, "shared:x", 0.6, 1.2, 4, 2, &[]),
                Err(Error::Duplicate)
            );
            assert_eq!(registrar.general_entity_types.len(), 1);
            assert_eq!(registrar.general_entity_types[0].mod_id, "amod");
            assert_eq!(registrar.refused.len(), 1);
        });
    }

    #[test]
    pub(crate) fn declare_general_entity_reaches_mods_through_dyn_registrar() {
        let _scope = PhaseOwner::scope(ModPhase::Registration);
        let mut registrar = PartRegistrar::new("mymod");
        let dynamic: &mut dyn crate::Registrar = &mut registrar;
        dynamic
            .declare_general_entity(
                "robot",
                GeneralEntityCategory::Misc,
                0.8,
                1.9,
                GeneralSavePolicy::Save,
                true,
                8,
                3,
                1,
                &[],
            )
            .unwrap();
        assert_eq!(registrar.general_entity_types.len(), 1);
    }
}
