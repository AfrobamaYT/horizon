//! The registries a server sends its clients (arismoko/horizon#811, stage 4, the engine-neutral
//! half): a snapshot of the synchronized registries of one published set, encoded with the
//! wire format, and the checks a client makes before it activates anything. Negotiating which
//! registries a peer gets and carrying the bytes are the game's. See
//! `docs/modding/src/concepts/mod-datapack-registries.md`.
use super::datapack_registry::{
    DatapackEntry, DatapackField, DatapackRegistryDeclaration, DatapackRegistrySync, DatapackValue,
    valid_namespaced,
};
use super::datapack_registry_set::{
    DatapackCandidate, DatapackLoadError, DatapackRegistrySet, resolve_references,
};
use crate::wire::{Reader, Wire, Writer};
use std::collections::{BTreeMap, BTreeSet};

/// The most bytes an encoded snapshot may take.
pub const MAX_SNAPSHOT_BYTES: usize = 64 << 20;
/// The most registries a snapshot may carry.
pub const MAX_SNAPSHOT_REGISTRIES: usize = 256;
/// The most entries one registry of a snapshot may carry.
pub const MAX_SNAPSHOT_ENTRIES: usize = 65_536;

/// One entry of a snapshot: its id and one value per field of the registry, in schema order.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: a plain pair that the snapshot builds and reads
#[derive(horizon_mod_api_macros::Wire)]
pub struct DatapackSnapshotEntry {
    /// The entry's id (`addon:fire`).
    pub id: String,
    /// The values, one per field of the registry's schema.
    pub values: Vec<DatapackValue>,
}

/// One registry of a snapshot: its key, the schema the server decoded it with, and its entries
/// sorted by id.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: a plain record that the snapshot builds and reads
#[derive(horizon_mod_api_macros::Wire)]
pub struct DatapackSnapshotRegistry {
    /// The registry's key (`example:spell`).
    pub key: String,
    /// The schema, which the client compares with its own declaration.
    pub fields: Vec<DatapackField>,
    /// The entries, sorted by id.
    pub entries: Vec<DatapackSnapshotEntry>,
}

/// The synchronized registries of a set at one generation, as a server sends them.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: a plain record that the snapshot builds and reads
#[derive(horizon_mod_api_macros::Wire)]
pub struct DatapackSnapshot {
    /// The server's world session.
    pub session: u64,
    /// The generation these entries belong to, from 1.
    pub generation: u64,
    /// The registries, sorted by key.
    pub registries: Vec<DatapackSnapshotRegistry>,
}

/// Why a snapshot was refused. A refused snapshot activates nothing.
#[derive(Clone, Debug, PartialEq)]
// no wire: the game reports it and disconnects
#[non_exhaustive]
pub enum DatapackSnapshotError {
    /// The bytes are not a snapshot, or are more than [`MAX_SNAPSHOT_BYTES`].
    Malformed,
    /// More registries or entries than [`MAX_SNAPSHOT_REGISTRIES`] and [`MAX_SNAPSHOT_ENTRIES`].
    TooLarge,
    /// The snapshot has a registry this side did not declare.
    UnknownRegistry {
        /// The registry's key.
        registry: String,
    },
    /// This side declared the registry with a different schema, or the snapshot lists it twice.
    SchemaMismatch {
        /// The registry's key.
        registry: String,
    },
    /// This side declared the registry as required and the snapshot has none of it.
    MissingRegistry {
        /// The registry's key.
        registry: String,
    },
    /// An entry is not a valid id, is listed twice, or has a value its field refuses.
    InvalidEntry {
        /// The registry's key.
        registry: String,
        /// The entry's id.
        entry: String,
    },
    /// A reference names an entry the snapshot does not have.
    Unresolved(Box<DatapackLoadError>),
}

impl DatapackRegistrySet {
    /// The snapshot of this set a peer receives: the registries `declarations` list as
    /// synchronized and `accepts` lets through (the game passes what the peer negotiated).
    /// Server-only registries are never in it.
    pub fn snapshot(
        &self,
        declarations: &[DatapackRegistryDeclaration],
        accepts: impl Fn(&DatapackRegistryDeclaration) -> bool,
    ) -> DatapackSnapshot {
        let mut registries: Vec<_> = declarations
            .iter()
            .filter(|declaration| declaration.sync != DatapackRegistrySync::ServerOnly)
            .filter(|declaration| accepts(declaration))
            .filter_map(|declaration| {
                let entries = self.registries.get(&declaration.key)?;
                Some(DatapackSnapshotRegistry {
                    key: declaration.key.clone(),
                    fields: declaration.fields.clone(),
                    entries: entries
                        .iter()
                        .map(|(id, entry)| DatapackSnapshotEntry {
                            id: id.clone(),
                            values: entry.values.iter().map(|(_, v)| v.clone()).collect(),
                        })
                        .collect(),
                })
            })
            .collect();
        registries.sort_by(|a, b| a.key.cmp(&b.key));
        DatapackSnapshot {
            session: self.session,
            generation: self.generation,
            registries,
        }
    }

    /// Publishes a snapshot's candidate on a client at the generation the server sent. The
    /// session is this client's own, so a handle from an earlier world fails as on a server.
    pub fn received(
        snapshot: &DatapackSnapshot,
        candidate: DatapackCandidate,
        session: u64,
    ) -> Self {
        Self {
            session,
            generation: snapshot.generation,
            registries: candidate.registries,
        }
    }
}

impl DatapackSnapshot {
    /// The wire bytes, or `None` when the snapshot is more than [`MAX_SNAPSHOT_BYTES`].
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut writer = Writer::new();
        writer.limit(MAX_SNAPSHOT_BYTES);
        self.put(&mut writer);
        writer.error().is_none().then(|| writer.into_bytes())
    }

    /// Reads wire bytes, refusing anything over [`MAX_SNAPSHOT_BYTES`], malformed or with
    /// bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, DatapackSnapshotError> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(DatapackSnapshotError::Malformed);
        }
        Reader::decode::<Self>(bytes).map_err(|_| DatapackSnapshotError::Malformed)
    }

    /// Checks the snapshot against this side's `declarations` and builds the candidate to
    /// activate: every registry must be one this side declared with the same schema, every
    /// value must fit its field, ids must be valid and unique, references must resolve, and a
    /// registry declared `Required` must be present. A declared registry the snapshot lacks
    /// (server-only, or optional and not negotiated) stays empty. Nothing is built unless
    /// everything passes.
    pub fn candidate(
        &self,
        declarations: &[DatapackRegistryDeclaration],
    ) -> Result<DatapackCandidate, DatapackSnapshotError> {
        if self.registries.len() > MAX_SNAPSHOT_REGISTRIES {
            return Err(DatapackSnapshotError::TooLarge);
        }
        let mut registries: BTreeMap<String, BTreeMap<String, DatapackEntry>> = declarations
            .iter()
            .map(|declaration| (declaration.key.clone(), BTreeMap::new()))
            .collect();
        let mut seen = BTreeSet::new();
        for sent in &self.registries {
            let registry = sent.key.clone();
            let declaration = declarations
                .iter()
                .find(|declaration| declaration.key == sent.key)
                .ok_or_else(|| DatapackSnapshotError::UnknownRegistry {
                    registry: registry.clone(),
                })?;
            if !seen.insert(sent.key.as_str()) || declaration.fields != sent.fields {
                return Err(DatapackSnapshotError::SchemaMismatch { registry });
            }
            if sent.entries.len() > MAX_SNAPSHOT_ENTRIES {
                return Err(DatapackSnapshotError::TooLarge);
            }
            let entries = registries.get_mut(&sent.key).expect("declared");
            for entry in &sent.entries {
                let invalid = || DatapackSnapshotError::InvalidEntry {
                    registry: registry.clone(),
                    entry: entry.id.clone(),
                };
                if !valid_namespaced(&entry.id)
                    || entry.values.len() != declaration.fields.len()
                    || !declaration
                        .fields
                        .iter()
                        .zip(&entry.values)
                        .all(|(field, value)| field.kind.accepts(value))
                {
                    return Err(invalid());
                }
                let values = declaration
                    .fields
                    .iter()
                    .map(|field| field.name.clone())
                    .zip(entry.values.iter().cloned())
                    .collect();
                if entries
                    .insert(entry.id.clone(), DatapackEntry { values })
                    .is_some()
                {
                    return Err(invalid());
                }
            }
        }
        if let Some(missing) = declarations.iter().find(|declaration| {
            declaration.sync == DatapackRegistrySync::Required
                && !seen.contains(declaration.key.as_str())
        }) {
            return Err(DatapackSnapshotError::MissingRegistry {
                registry: missing.key.clone(),
            });
        }
        resolve_references(declarations, &registries).map_err(DatapackSnapshotError::Unresolved)?;
        Ok(DatapackCandidate { registries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::datapack_registry::DatapackFieldKind;

    fn spell(sync: DatapackRegistrySync) -> DatapackRegistryDeclaration {
        DatapackRegistryDeclaration {
            mod_id: "example".into(),
            key: "example:spell".into(),
            fields: vec![
                DatapackField::required("power", DatapackFieldKind::Int { min: 0, max: 100 }),
                DatapackField::with_default(
                    "next",
                    DatapackFieldKind::Reference {
                        registry: "example:spell".into(),
                    },
                    DatapackValue::Text("addon:fire".into()),
                ),
            ],
            sync,
        }
    }

    fn set(declaration: &DatapackRegistryDeclaration) -> DatapackRegistrySet {
        let files = vec![
            (
                "data/addon/example/spell/fire.json".to_owned(),
                vec![br#"{"power": 7}"#.to_vec()],
            ),
            (
                "data/addon/example/spell/ice.json".to_owned(),
                vec![br#"{"power": 4, "next": "addon:fire"}"#.to_vec()],
            ),
        ];
        let candidate =
            DatapackCandidate::build(std::slice::from_ref(declaration), &files).unwrap();
        DatapackRegistrySet::first(candidate, 1)
    }

    #[test]
    fn a_client_reads_what_the_server_sent_at_the_same_generation() {
        let declaration = spell(DatapackRegistrySync::Required);
        let server = set(&declaration);
        let server = server
            .reload(
                DatapackCandidate::build(
                    std::slice::from_ref(&declaration),
                    &[(
                        "data/addon/example/spell/fire.json".to_owned(),
                        vec![br#"{"power": 9}"#.to_vec()],
                    )],
                )
                .unwrap(),
            )
            .unwrap();
        let snapshot = server.snapshot(std::slice::from_ref(&declaration), |_| true);
        let bytes = snapshot.encode().unwrap();
        let received = DatapackSnapshot::decode(&bytes).unwrap();
        assert_eq!(received, snapshot);
        let candidate = received
            .candidate(std::slice::from_ref(&declaration))
            .unwrap();
        let client = DatapackRegistrySet::received(&received, candidate, 99);
        assert_eq!(client.generation(), 2);
        let handle = client.handle("example:spell", "addon:fire").unwrap();
        assert_eq!(
            client.get(&handle).unwrap().get("power"),
            Some(&DatapackValue::Int(9))
        );
    }

    #[test]
    fn server_only_and_unaccepted_registries_are_not_sent() {
        let only = spell(DatapackRegistrySync::ServerOnly);
        assert!(
            set(&only)
                .snapshot(std::slice::from_ref(&only), |_| true)
                .registries
                .is_empty()
        );
        let optional = spell(DatapackRegistrySync::Optional);
        assert!(
            set(&optional)
                .snapshot(std::slice::from_ref(&optional), |_| false)
                .registries
                .is_empty()
        );
    }

    #[test]
    fn a_client_refuses_a_snapshot_that_does_not_match_its_declarations() {
        let declaration = spell(DatapackRegistrySync::Required);
        let snapshot = set(&declaration).snapshot(std::slice::from_ref(&declaration), |_| true);
        // A registry this side never declared.
        assert_eq!(
            snapshot.candidate(&[]).unwrap_err(),
            DatapackSnapshotError::UnknownRegistry {
                registry: "example:spell".into()
            }
        );
        // A different schema.
        let mut other = declaration.clone();
        other.fields.pop();
        assert_eq!(
            snapshot.candidate(&[other]).unwrap_err(),
            DatapackSnapshotError::SchemaMismatch {
                registry: "example:spell".into()
            }
        );
        // A required registry the snapshot lacks.
        let empty = DatapackSnapshot {
            session: 1,
            generation: 1,
            registries: Vec::new(),
        };
        assert_eq!(
            empty
                .candidate(std::slice::from_ref(&declaration))
                .unwrap_err(),
            DatapackSnapshotError::MissingRegistry {
                registry: "example:spell".into()
            }
        );
        // An optional one it lacks stays empty.
        let optional = spell(DatapackRegistrySync::Optional);
        let candidate = empty.candidate(std::slice::from_ref(&optional)).unwrap();
        let client = DatapackRegistrySet::received(&empty, candidate, 2);
        assert_eq!(client.entries("example:spell").unwrap(), Vec::<&str>::new());
    }

    #[test]
    fn a_client_refuses_bad_entries_and_dangling_references() {
        let declaration = spell(DatapackRegistrySync::Required);
        let good = set(&declaration).snapshot(std::slice::from_ref(&declaration), |_| true);
        let refuses = |change: &dyn Fn(&mut DatapackSnapshot)| {
            let mut snapshot = good.clone();
            change(&mut snapshot);
            snapshot
                .candidate(std::slice::from_ref(&declaration))
                .unwrap_err()
        };
        // A value past its field's bound.
        assert!(matches!(
            refuses(&|s| s.registries[0].entries[0].values[0] = DatapackValue::Int(101)),
            DatapackSnapshotError::InvalidEntry { .. }
        ));
        // The wrong number of values.
        assert!(matches!(
            refuses(&|s| {
                s.registries[0].entries[0].values.pop();
            }),
            DatapackSnapshotError::InvalidEntry { .. }
        ));
        // A bad id, and an id listed twice.
        assert!(matches!(
            refuses(&|s| s.registries[0].entries[0].id = "NoColon".into()),
            DatapackSnapshotError::InvalidEntry { .. }
        ));
        assert!(matches!(
            refuses(&|s| {
                let first = s.registries[0].entries[0].clone();
                s.registries[0].entries.push(first);
            }),
            DatapackSnapshotError::InvalidEntry { .. }
        ));
        // A reference to an entry the snapshot does not have.
        assert!(matches!(
            refuses(&|s| {
                s.registries[0].entries[1].values[1] = DatapackValue::Text("addon:gone".into())
            }),
            DatapackSnapshotError::Unresolved(_)
        ));
        // The same registry twice.
        assert!(matches!(
            refuses(&|s| {
                let first = s.registries[0].clone();
                s.registries.push(first);
            }),
            DatapackSnapshotError::SchemaMismatch { .. }
        ));
    }

    #[test]
    fn bytes_that_are_not_a_snapshot_are_refused() {
        let declaration = spell(DatapackRegistrySync::Required);
        let snapshot = set(&declaration).snapshot(std::slice::from_ref(&declaration), |_| true);
        let mut bytes = snapshot.encode().unwrap();
        bytes.push(0);
        assert_eq!(
            DatapackSnapshot::decode(&bytes),
            Err(DatapackSnapshotError::Malformed)
        );
        bytes.truncate(bytes.len() - 3);
        assert_eq!(
            DatapackSnapshot::decode(&bytes),
            Err(DatapackSnapshotError::Malformed)
        );
    }
}
