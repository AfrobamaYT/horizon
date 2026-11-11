//! General entity rows (#792 stage 2): the record codec for mod-declared
//! entity types. The record is the save format's unit: common fields every
//! kind shares, plus the type envelope (namespaced key, payload schema
//! version, payload bytes) and the passenger tree. Validation runs every
//! check before any of it may reach a row; a refusal names the first fault
//! and never panics, clamps or truncates into a valid-looking entity.

/// The design's bounds, re-exported from the one `limits` block the
/// registrar quotes (`mod_api::general_entity_limits`).
pub use mod_api::general_entity_limits as limits;

use super::Game;
use super::commands::Target;
use super::entity::Entity;
use super::entity_initialization::TransferredEntityState;
use super::persist::ChunkKey;
use horizon_client::entity_rules::EntityRules;
use horizon_client::impl_joined;
use horizon_client::join_seq::{JoinList, JoinSeq};
use horizon_core::fast_hash::{FxHashMap, FxHashSet};
use horizon_core::geometry::{Aabb, Vec3};
use horizon_core::nbt::Tag;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::LazyLock;

/// One general entity as a record: what save writes and load reads. The
/// engine's live row (`GeneralRow`, stage 3) is built from a validated
/// record; this type never holds runtime ids, slots or pointers.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralEntityRecord {
    /// The entity's position in blocks.
    pub position: [f64; 3],
    /// The entity's velocity in blocks per tick.
    pub velocity: [f64; 3],
    /// The entity's yaw and pitch in degrees.
    pub rotation: [f32; 2],
    /// The entity's UUID, the identity a reload continues.
    pub uuid: [i32; 4],
    /// The type envelope: which registered type owns the payload, at which
    /// schema version, with which bounded payload bytes.
    pub envelope: GeneralTypeEnvelope,
    /// The entities riding this one, stored inside its record.
    pub passengers: Vec<GeneralEntityRecord>,
}

/// The mod-owned half of a record: the registered type's namespaced key, the
/// payload's schema version and the payload bytes themselves.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralTypeEnvelope {
    /// The type's namespaced id (`mymod:robot`), as declared.
    pub type_key: String,
    /// The payload's schema version, as declared by the type.
    pub payload_version: u16,
    /// The payload bytes, at most [`limits::PAYLOAD_LIMIT`] long.
    pub payload_bytes: Vec<u8>,
}

#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
/// Why a general entity record was refused before any of it could reach a
/// row. Each fault names the record's uuid in its diagnostic string.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GeneralRecordError {
    /// A mod payload exceeds [`limits::PAYLOAD_LIMIT`]. The bound applies to
    /// mod payloads only, never to a record of a vanilla type.
    PayloadTooLarge { size: usize },
    /// The decoded data of a payload is nested deeper than
    /// [`limits::MAX_NESTING`]. Stage 3's payload decode owns this check —
    /// the record walk never fires it, because a passenger tree reaches
    /// [`GeneralRecordError::PassengerDepthTooDeep`] first.
    NestingTooDeep { depth: u32 },
    /// A passenger chain is deeper than [`limits::MAX_PASSENGER_DEPTH`].
    PassengerDepthTooDeep { depth: u32 },
    /// A passenger's UUID is one of its own ancestors: a cycle.
    PassengerCycle { uuid: [i32; 4] },
    /// The same UUID appears twice in one record tree.
    DuplicateUuid { uuid: [i32; 4] },
    /// A position, velocity or rotation component is not finite.
    NotFinite,
}

#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
impl GeneralRecordError {
    /// The refusal's diagnostic, for the load summary.
    pub fn message(&self) -> String {
        match self {
            GeneralRecordError::PayloadTooLarge { size } => format!(
                "general entity payload {size} exceeds {}",
                limits::PAYLOAD_LIMIT
            ),
            GeneralRecordError::NestingTooDeep { depth } => format!(
                "general entity nesting {depth} exceeds {}",
                limits::MAX_NESTING
            ),
            GeneralRecordError::PassengerDepthTooDeep { depth } => format!(
                "general entity passenger depth {depth} exceeds {}",
                limits::MAX_PASSENGER_DEPTH
            ),
            GeneralRecordError::PassengerCycle { uuid } => {
                format!("general entity passenger cycle at {uuid:?}")
            }
            GeneralRecordError::DuplicateUuid { uuid } => {
                format!("general entity duplicate uuid {uuid:?}")
            }
            GeneralRecordError::NotFinite => {
                "general entity has a non-finite coordinate".to_owned()
            }
        }
    }
}

#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
/// The outcome of checking a record against every bound: ready to become a
/// row, or structurally sound but of a type this load cannot resolve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralRecordCheck {
    /// Every check passed; the record may reach a row.
    Ready,
    /// Every structural check passed, but the type key's namespace belongs
    /// to neither vanilla nor a loaded mod. Stage 3 keeps such a record
    /// byte for byte as quarantined data; it never reaches a row here.
    UnknownType,
}

/// Check one record tree against every bound, before any of it may reach a
/// row. `is_known_type` answers whether a type key's namespace belongs to
/// vanilla or a loaded mod; the caller supplies the frozen registry's view.
///
/// Structural faults (payload size, passenger depth, cycles,
/// duplicate UUIDs, non-finite coordinates) refuse the tree whatever its
/// type; only a structurally sound record of an unresolvable type reports
/// [`GeneralRecordCheck::UnknownType`], so quarantine never preserves an
/// over-budget or malformed payload. The design's separate data-nesting
/// bound is the payload decode's to check (stage 3), not this walk's: a
/// passenger tree reaches the passenger-depth refusal first. Nothing is
/// written, allocated beyond the walk's own bookkeeping, or panicked on; a
/// refused tree leaves no partial state, and the next valid record checks
/// cleanly.
#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
pub fn check_general_record(
    record: &GeneralEntityRecord,
    is_known_type: impl Fn(&str) -> bool,
) -> Result<GeneralRecordCheck, GeneralRecordError> {
    let mut seen = FxHashSet::default();
    check_record(record, &mut Vec::new(), &mut seen, 1, &is_known_type)
}

#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
fn check_record(
    record: &GeneralEntityRecord,
    ancestors: &mut Vec<[i32; 4]>,
    seen: &mut FxHashSet<[i32; 4]>,
    depth: u32,
    is_known_type: &impl Fn(&str) -> bool,
) -> Result<GeneralRecordCheck, GeneralRecordError> {
    if !record
        .position
        .iter()
        .chain(record.velocity.iter())
        .all(|component| component.is_finite())
        || !record
            .rotation
            .iter()
            .all(|component| component.is_finite())
    {
        return Err(GeneralRecordError::NotFinite);
    }
    if record.envelope.payload_bytes.len() > limits::PAYLOAD_LIMIT {
        return Err(GeneralRecordError::PayloadTooLarge {
            size: record.envelope.payload_bytes.len(),
        });
    }
    if depth - 1 > limits::MAX_PASSENGER_DEPTH {
        return Err(GeneralRecordError::PassengerDepthTooDeep { depth: depth - 1 });
    }
    if ancestors.contains(&record.uuid) {
        return Err(GeneralRecordError::PassengerCycle { uuid: record.uuid });
    }
    if !seen.insert(record.uuid) {
        return Err(GeneralRecordError::DuplicateUuid { uuid: record.uuid });
    }
    ancestors.push(record.uuid);
    let mut unknown = false;
    for passenger in &record.passengers {
        match check_record(passenger, ancestors, seen, depth + 1, is_known_type)? {
            GeneralRecordCheck::Ready => {}
            GeneralRecordCheck::UnknownType => unknown = true,
        }
    }
    ancestors.pop();
    if unknown || !is_known_type(&record.envelope.type_key) {
        return Ok(GeneralRecordCheck::UnknownType);
    }
    Ok(GeneralRecordCheck::Ready)
}

/// The frozen type registry: every declared general entity type, resolved
/// once into dense slots. The registry is data — built from the installed
/// declarations and never hand-kept — and freezes with them: duplicates
/// across mods refuse at freeze, naming the type.
#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
pub struct GeneralTypeCatalog {
    entries: Vec<mod_api::__private::GeneralEntityTypeDeclaration>,
    by_key: FxHashMap<String, u16>,
}

impl GeneralTypeCatalog {
    /// Freeze the installed declarations into slots, sorted by declaring
    /// mod id then type id, so every side running the same mods builds the
    /// same slot numbers. A duplicate type key refuses, naming it.
    pub fn freeze(
        declarations: &[mod_api::__private::GeneralEntityTypeDeclaration],
    ) -> Result<Self, String> {
        let mut sorted = declarations.to_vec();
        sorted.sort_by(|left, right| (&left.mod_id, &left.id).cmp(&(&right.mod_id, &right.id)));
        let mut by_key = FxHashMap::with_capacity_and_hasher(sorted.len(), Default::default());
        for (slot, declaration) in sorted.iter().enumerate() {
            let slot = slot as u16;
            if by_key.insert(declaration.id.clone(), slot).is_some() {
                return Err(format!(
                    "duplicate general entity type {} (declared by {})",
                    declaration.id, declaration.mod_id
                ));
            }
        }
        Ok(Self {
            entries: sorted,
            by_key,
        })
    }

    #[allow(
        dead_code,
        reason = "stage 3's load path and the row's construction are the first non-test users"
    )]
    /// The slot a type key resolved to, if the registry holds it.
    pub fn slot(&self, type_key: &str) -> Option<u16> {
        self.by_key.get(type_key).copied()
    }

    /// The type key a slot belongs to.
    pub fn key(&self, slot: u16) -> &str {
        &self.entries[usize::from(slot)].id
    }

    /// The type's declared width and height, for the row's bounds.
    pub fn dimensions(&self, slot: u16) -> (f64, f64) {
        let entry = &self.entries[usize::from(slot)];
        (entry.width, entry.height)
    }
}

/// The process's frozen registry, built once from the installed
/// declarations: the types were declared before any world or connection
/// opened, and the registry never changes after.
pub fn general_type_catalog() -> &'static GeneralTypeCatalog {
    static CATALOG: LazyLock<GeneralTypeCatalog> = LazyLock::new(|| {
        GeneralTypeCatalog::freeze(mod_api::__private::general_entity_type_declarations())
            .unwrap_or_else(|duplicate| {
                // Unreachable: a duplicate key was already refused at
                // registration — every mod declares through the one
                // registrar `register_all` runs (`parts/shared.rs`), so the
                // second mod's `declare_general_entity` sees the first's
                // key and returns `Duplicate`, never reaching here.
                unreachable!("duplicate general entity type at freeze: {duplicate}")
            })
    });
    &CATALOG
}

/// The engine's live general rows (#792): identity, join identity, type
/// slot, common state and the attachment holder. No runtime slot or pointer
/// persists; identity-to-row lookup stays with `EntityTable`
/// (`entity_lookup`, `server_entity_lookup`), never a second index here.
#[derive(Clone, Debug)]
pub struct GeneralRow {
    /// The row's runtime network id, from the game's one counter.
    pub id: u32,
    #[allow(
        dead_code,
        reason = "stage 3's load path and the row's construction are the first non-test users"
    )]
    /// The row's UUID; a reload continues the identity, not old handles.
    pub uuid: [i32; 4],
    /// The row's join identity: its admission into the resident table.
    pub join: JoinSeq,
    /// The frozen registry's slot for the row's type.
    pub type_slot: u16,
    /// The position in blocks.
    pub position: Vec3,
    /// Where the row stood when the last tick began.
    pub previous: Vec3,
    /// The velocity in blocks per tick.
    pub velocity: Vec3,
    /// The yaw and pitch in degrees.
    pub rotation: [f32; 2],
    /// How far the row has fallen; stage 3's load and stage 4's tick own it.
    pub fall_distance: f64,
    /// The entity-owned attachment registration lifetime (#796: the sole
    /// Part-data store; there is no parallel entity data map).
    pub attachment_lifetime: horizon_mod_runtime::attachments::AttachmentLifetime,
}

impl EntityRules for GeneralRow {}
impl_joined!(GeneralRow);
horizon_mod_runtime::impl_attachment_lifetime!(GeneralRow);

/// The general rows' list, one per level, bound to residency like the other
/// families' lists.
pub type GeneralRows = JoinList<GeneralRow, true>;

/// The game's general entity store: one join list per level, wired once
/// into `resident_owners!` and `entity_lists!` as `Kind::General`.
#[derive(Default)]
pub struct GeneralStore {
    /// The level's general rows, in join order.
    pub rows: GeneralRows,
}

impl Entity for GeneralRow {
    fn id(&self) -> u32 {
        self.id
    }
    fn position(&self) -> Vec3 {
        self.position
    }
    fn position_mut(&mut self) -> &mut Vec3 {
        &mut self.position
    }
    fn previous_position(&self) -> Vec3 {
        self.previous
    }
    fn velocity(&self) -> Vec3 {
        self.velocity
    }
    fn velocity_mut(&mut self) -> &mut Vec3 {
        &mut self.velocity
    }
    fn reset_fall_distance(&mut self) {
        self.fall_distance = 0.0;
    }
    fn rotation(&self) -> [f32; 2] {
        self.rotation
    }
    fn set_rotation(&mut self, rotation: [f32; 2]) {
        self.rotation = rotation;
    }
    fn bounds(&self) -> Aabb {
        let (width, height) = general_type_catalog().dimensions(self.type_slot);
        let half = width / 2.0;
        let at = self.position;
        Aabb::new(
            Vec3::new(at.x - half, at.y, at.z - half),
            Vec3::new(at.x + half, at.y + height, at.z + half),
        )
    }
    fn selector_target(&self) -> Option<Target> {
        // Stage 2 rows never join a world; a selector cannot name one yet.
        None
    }
    fn native_type(&self) -> Cow<'static, str> {
        Cow::Owned(general_type_catalog().key(self.type_slot).to_owned())
    }
    fn save_nbt(
        _game: &mut Game<'_>,
        _index: usize,
        _keys: &HashSet<ChunkKey>,
    ) -> Option<(ChunkKey, Tag)> {
        // Stage 3's save owns the general record; no row is saved here.
        None
    }
    fn load_nbt(
        _game: &mut Game<'_>,
        _kind: &str,
        _uuid: Option<[i32; 4]>,
        _player_uuid: Option<[i32; 4]>,
        tag: Tag,
        _transferred: &mut Option<TransferredEntityState>,
    ) -> Result<(), Tag> {
        // Stage 3's load owns the general record; the tag is handed back.
        Err(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(uuid: [i32; 4], type_key: &str) -> GeneralEntityRecord {
        GeneralEntityRecord {
            position: [0.0, 64.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0],
            uuid,
            envelope: GeneralTypeEnvelope {
                type_key: type_key.to_owned(),
                payload_version: 1,
                payload_bytes: Vec::new(),
            },
            passengers: Vec::new(),
        }
    }

    fn known(type_key: &str) -> impl Fn(&str) -> bool {
        let owned = type_key.to_owned();
        move |key| key == owned
    }

    /// A minimal valid tree checks ready, whatever its passengers' order.
    #[test]
    fn valid_record_with_passengers_checks_ready() {
        let mut root = record([1, 2, 3, 4], "mymod:robot");
        let mut rider = record([5, 6, 7, 8], "mymod:robot");
        rider
            .passengers
            .push(record([9, 10, 11, 12], "mymod:robot"));
        root.passengers.push(rider);
        assert_eq!(
            check_general_record(&root, known("mymod:robot")),
            Ok(GeneralRecordCheck::Ready)
        );
    }

    /// A mod payload one byte over the bound refuses with its size; at the
    /// bound it stands. Vanilla records never pass through this codec, so
    /// the bound cannot clip them.
    #[test]
    fn payload_bound_refuses_one_byte_over() {
        let mut over = record([1, 2, 3, 4], "mymod:robot");
        over.envelope.payload_bytes = vec![0; limits::PAYLOAD_LIMIT + 1];
        assert_eq!(
            check_general_record(&over, known("mymod:robot")),
            Err(GeneralRecordError::PayloadTooLarge {
                size: limits::PAYLOAD_LIMIT + 1
            })
        );
        let mut at = record([5, 6, 7, 8], "mymod:robot");
        at.envelope.payload_bytes = vec![0; limits::PAYLOAD_LIMIT];
        assert_eq!(
            check_general_record(&at, known("mymod:robot")),
            Ok(GeneralRecordCheck::Ready)
        );
    }

    /// A passenger chain one ride deeper than the passenger bound refuses,
    /// the root's own level excluded from the count. The design's separate
    /// data-nesting bound is stage 3's payload decode; this walk never
    /// reaches it, because this refusal fires first.
    #[test]
    fn passenger_depth_bound_refuses_past_the_limit() {
        let mut root = record([1, 2, 3, 4], "mymod:robot");
        let mut node = &mut root;
        for index in 0..limits::MAX_PASSENGER_DEPTH + 1 {
            node.passengers
                .push(record([index as i32 + 1, 0, 0, 0], "mymod:robot"));
            node = node.passengers.last_mut().unwrap();
        }
        assert_eq!(
            check_general_record(&root, known("mymod:robot")),
            Err(GeneralRecordError::PassengerDepthTooDeep {
                depth: limits::MAX_PASSENGER_DEPTH + 1
            })
        );
    }

    /// A passenger carrying its own ancestor's UUID is a cycle, refused
    /// wherever in the tree the repeat hides.
    #[test]
    fn passenger_cycle_refuses() {
        let mut root = record([1, 2, 3, 4], "mymod:robot");
        let mut rider = record([5, 6, 7, 8], "mymod:robot");
        // The rider's passenger is the root again.
        rider.passengers.push(record([1, 2, 3, 4], "mymod:robot"));
        root.passengers.push(rider);
        assert_eq!(
            check_general_record(&root, known("mymod:robot")),
            Err(GeneralRecordError::PassengerCycle { uuid: [1, 2, 3, 4] })
        );
    }

    /// Two records of one UUID in one tree refuse, siblings included.
    #[test]
    fn duplicate_uuid_refuses() {
        let mut root = record([1, 2, 3, 4], "mymod:robot");
        root.passengers.push(record([5, 6, 7, 8], "mymod:robot"));
        root.passengers.push(record([5, 6, 7, 8], "mymod:robot"));
        assert_eq!(
            check_general_record(&root, known("mymod:robot")),
            Err(GeneralRecordError::DuplicateUuid { uuid: [5, 6, 7, 8] })
        );
    }

    /// A NaN or infinite position, velocity or rotation component refuses
    /// before anything else is consulted.
    #[test]
    fn non_finite_coordinates_refuse() {
        for (slot, bad) in [(0, f64::NAN), (1, f64::NEG_INFINITY), (2, f64::NAN)] {
            let mut record = record([1, 2, 3, 4], "mymod:robot");
            record.position[slot] = bad;
            assert_eq!(
                check_general_record(&record, known("mymod:robot")),
                Err(GeneralRecordError::NotFinite)
            );
        }
        let mut velocity = record([1, 2, 3, 4], "mymod:robot");
        velocity.velocity[1] = f64::INFINITY;
        assert_eq!(
            check_general_record(&velocity, known("mymod:robot")),
            Err(GeneralRecordError::NotFinite)
        );
    }

    /// A structurally sound record of a type whose namespace belongs to
    /// neither vanilla nor a loaded mod reports `UnknownType`, the outcome
    /// stage 3 keeps as quarantined data; a structural fault in the same
    /// tree still refuses, so quarantine never preserves a bad payload.
    #[test]
    fn unknown_type_is_its_own_outcome() {
        let unknown = record([1, 2, 3, 4], "gone:robot");
        assert_eq!(
            check_general_record(&unknown, known("mymod:robot")),
            Ok(GeneralRecordCheck::UnknownType)
        );
        // A passenger of an unknown type poisons the root's readiness too:
        // the tree keeps together.
        let mut mixed = record([5, 6, 7, 8], "mymod:robot");
        mixed.passengers.push(record([9, 10, 11, 12], "gone:robot"));
        assert_eq!(
            check_general_record(&mixed, known("mymod:robot")),
            Ok(GeneralRecordCheck::UnknownType)
        );
        // But a structural fault refuses even for the unknown type.
        let mut over = record([13, 14, 15, 16], "gone:robot");
        over.envelope.payload_bytes = vec![0; limits::PAYLOAD_LIMIT + 1];
        assert_eq!(
            check_general_record(&over, known("mymod:robot")),
            Err(GeneralRecordError::PayloadTooLarge {
                size: limits::PAYLOAD_LIMIT + 1
            })
        );
    }

    fn declaration(mod_id: &str, id: &str) -> mod_api::__private::GeneralEntityTypeDeclaration {
        mod_api::__private::GeneralEntityTypeDeclaration {
            mod_id: mod_id.to_owned(),
            id: id.to_owned(),
            category: mod_api::GeneralEntityCategory::Misc,
            width: 0.8,
            height: 1.9,
            save_policy: mod_api::GeneralSavePolicy::Save,
            summonable: true,
            tracking_chunks: 8,
            update_interval: 3,
            payload_version: 1,
            required_features: Vec::new(),
        }
    }

    /// The frozen registry resolves slots in one stable order — declaring
    /// mod id, then type id — and a duplicate type key refuses at freeze,
    /// naming it, so two mods cannot claim one slot number.
    #[test]
    fn catalog_freezes_slots_in_declaration_order_and_refuses_duplicates() {
        let catalog = GeneralTypeCatalog::freeze(&[
            declaration("zmod", "zmod:zeal"),
            declaration("amod", "amod:alpha"),
            declaration("amod", "amod:beta"),
        ])
        .unwrap();
        assert_eq!(catalog.key(0), "amod:alpha");
        assert_eq!(catalog.key(1), "amod:beta");
        assert_eq!(catalog.key(2), "zmod:zeal");
        assert_eq!(catalog.slot("amod:beta"), Some(1));
        assert_eq!(catalog.dimensions(2), (0.8, 1.9));
        match GeneralTypeCatalog::freeze(&[
            declaration("amod", "amod:same"),
            declaration("bmod", "amod:same"),
        ]) {
            Err(refusal) => assert!(refusal.contains("duplicate general entity type amod:same")),
            Ok(_) => panic!("a duplicate type key froze"),
        }
    }

    /// A general row pushed to its list binds residency the way
    /// `resident_owners!` wires `Kind::General` (the default arm's
    /// `bind_residency` with the row's id and join identity), and the entity
    /// table then resolves the row's id to that owner: the one lookup path
    /// every family shares, with no second index.
    #[test]
    fn general_row_binds_and_resolves_through_the_entity_table() {
        use horizon_client::join_seq::residency::table::{EntityLookup, EntityTable, SourceEntity};
        use horizon_client::join_seq::residency::{Journal, Kind, Owner};
        use mod_api::DimensionKey;
        let journal = Journal::default();
        let mut rows: GeneralRows = Default::default();
        let owner = Owner {
            dimension: DimensionKey::OVERWORLD,
            kind: Kind::General,
        };
        rows.bind_residency(&journal, owner, |row| (row.id, row.join));
        rows.push(GeneralRow {
            id: 42,
            uuid: [1, 2, 3, 4],
            join: horizon_client::join_seq::JoinSeq::UNJOINED,
            type_slot: 0,
            position: Vec3::new(0.5, 64.0, 0.5),
            previous: Vec3::new(0.5, 64.0, 0.5),
            velocity: Vec3::ZERO,
            rotation: [0.0, 0.0],
            fall_distance: 0.0,
            attachment_lifetime: Default::default(),
        });
        let mut table = EntityTable::new(journal);
        let EntityLookup::Found { location, .. } =
            table.entity_lookup(&DimensionKey::OVERWORLD, 42, |at| {
                rows.locate(at.join).and_then(|index| {
                    rows.get(index).map(|row| SourceEntity {
                        id: row.id,
                        value: index,
                    })
                })
            })
        else {
            panic!("the general row never became resident");
        };
        assert_eq!(location.owner, owner);
        // The list keeps join order: removal is not a swap, and the row's
        // slot moves with it.
        let index = rows.len() - 1;
        assert_eq!(rows[index].id, 42);
        assert_ne!(
            rows[index].join,
            horizon_client::join_seq::JoinSeq::UNJOINED
        );
    }

    /// A refusal leaves no partial state: the very next valid record checks
    /// cleanly through the same entry point.
    #[test]
    fn refused_record_does_not_poison_the_next_one() {
        let mut over = record([1, 2, 3, 4], "mymod:robot");
        over.envelope.payload_bytes = vec![0; limits::PAYLOAD_LIMIT + 1];
        assert!(check_general_record(&over, known("mymod:robot")).is_err());
        let good = record([5, 6, 7, 8], "mymod:robot");
        assert_eq!(
            check_general_record(&good, known("mymod:robot")),
            Ok(GeneralRecordCheck::Ready)
        );
    }
}
