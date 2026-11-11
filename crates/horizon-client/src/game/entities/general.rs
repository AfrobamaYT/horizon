//! General entity rows (#792 stage 2): the record codec for mod-declared
//! entity types. The record is the save format's unit: common fields every
//! kind shares, plus the type envelope (namespaced key, payload schema
//! version, payload bytes) and the passenger tree. Validation runs every
//! check before any of it may reach a row; a refusal names the first fault
//! and never panics, clamps or truncates into a valid-looking entity.

/// The design's bounds, re-exported from the one `limits` block the
/// registrar quotes (`mod_api::general_entity_limits`).
pub use mod_api::general_entity_limits as limits;

use horizon_client::join_seq::JoinList;
use horizon_core::fast_hash::FxHashSet;

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
    /// The record tree is deeper than [`limits::MAX_NESTING`], root included.
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
/// Structural faults (payload size, nesting, passenger depth, cycles,
/// duplicate UUIDs, non-finite coordinates) refuse the tree whatever its
/// type; only a structurally sound record of an unresolvable type reports
/// [`GeneralRecordCheck::UnknownType`], so quarantine never preserves an
/// over-budget or malformed payload. Nothing is written, allocated beyond
/// the walk's own bookkeeping, or panicked on; a refused tree leaves no
/// partial state, and the next valid record checks cleanly.
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
    if depth > limits::MAX_NESTING {
        return Err(GeneralRecordError::NestingTooDeep { depth });
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

#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
/// The engine's live general rows (#792 stage 3 target): identity, join
/// identity, type slot, common state and the attachment holder, in one
/// dense per-level list bound to residency like every other family. Stage 2
/// declares the shape so the codec and the tests share its field types; the
/// store itself joins `resident_owners!` and `entity_lists!` in stage 3.
#[derive(Clone, Debug)]
pub struct GeneralRow {
    /// The row's runtime network id, from the game's one counter.
    pub id: u32,
    /// The row's UUID; a reload continues the identity, not old handles.
    pub uuid: [i32; 4],
    /// The row's join identity: its admission into the resident table.
    pub join: horizon_client::join_seq::JoinSeq,
    /// The position in blocks.
    pub position: [f64; 3],
    /// The velocity in blocks per tick.
    pub velocity: [f64; 3],
    /// The yaw and pitch in degrees.
    pub rotation: [f32; 2],
    /// The entity-owned attachment registration lifetime (#796: the sole
    /// Part-data store; there is no parallel entity data map).
    pub attachment_lifetime: horizon_mod_runtime::attachments::AttachmentLifetime,
}

/// The general rows' list, one per level, bound to residency like the other
/// families' lists. Identity-to-row lookup stays with `EntityTable`
/// (`entity_lookup`, `server_entity_lookup`), never a second index here.
#[allow(
    dead_code,
    reason = "stage 3's load path and the row's construction are the first non-test users"
)]
pub type GeneralRows = JoinList<GeneralRow, true>;

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

    /// A tree one level deeper than the nesting bound refuses at that depth.
    /// The passenger bound (16) is stricter than the nesting bound (32), so
    /// no valid passenger tree can reach this fault; the branch guards the
    /// record shape stage 3 grows (nested non-passenger state) and is checked
    /// where only it can fire.
    #[test]
    fn nesting_bound_refuses_past_the_limit() {
        let record = record([1, 2, 3, 4], "mymod:robot");
        let mut ancestors = Vec::new();
        let mut seen = FxHashSet::default();
        seen.insert([1, 2, 3, 4]);
        assert_eq!(
            check_record(
                &record,
                &mut ancestors,
                &mut seen,
                limits::MAX_NESTING + 1,
                &known("mymod:robot"),
            ),
            Err(GeneralRecordError::NestingTooDeep {
                depth: limits::MAX_NESTING + 1
            })
        );
    }

    /// A passenger chain one ride deeper than the passenger bound refuses,
    /// the root's own level excluded from the count.
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
