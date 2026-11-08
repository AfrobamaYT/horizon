//! Registration-time resolution of declared internals into endpoint exposure plans.
use crate::exposure::{Grant, Layout, Plan};
use std::sync::Arc;

/// A point's virtual call layouts and special receiver layout.
#[derive(Clone, Debug)]
pub struct PointSpec {
    /// Head virtual call layout.
    pub head_call: usize,
    /// Tail virtual call layout, if supported.
    pub tail_call: Option<usize>,
    /// Receiver layout, which remains partial even through backreferences.
    pub receiver: Option<usize>,
}

/// One exact catalog entry which grants a receiver field on a particular layout.
#[derive(Clone, Debug)]
pub struct InternalField {
    /// Canonical internal path.
    pub path: String,
    /// Catalog member kind.
    pub kind: String,
    /// Exact catalog hash.
    pub hash: String,
    /// Receiver layout index.
    pub receiver: usize,
    /// Field name in the receiver layout.
    pub field: String,
}

/// Pure metadata emitted from the same graph as the native readers.
#[derive(Clone, Debug)]
pub struct SchemaCatalog {
    /// Ordered graph layouts.
    pub layouts: Vec<Layout>,
    /// Point metadata indexed by raw point index.
    pub points: Vec<PointSpec>,
    /// Exact catalog receiver-field entries.
    pub fields: Vec<InternalField>,
}

/// A manifest internal declaration, independent of package DTOs.
#[derive(Clone, Debug)]
pub struct InternalDeclaration {
    /// Canonical internal path.
    pub path: String,
    /// Optional catalog kind; absent requires an unambiguous path.
    pub kind: Option<String>,
    /// Pinned catalog hash.
    pub hash: String,
}

/// Manifest declarations to resolve before endpoint registration.
pub type ManifestInternals = [InternalDeclaration];

/// Module-owned precomputed policies. Dropping this releases its plan owners.
pub struct EndpointPolicy {
    plans: std::collections::HashMap<usize, Arc<Plan>>,
}
impl EndpointPolicy {
    /// Validate every declaration and build all point policies atomically.
    pub fn build(
        catalog: &SchemaCatalog,
        internals: &ManifestInternals,
        listened: &[usize],
    ) -> Result<Self, String> {
        let mut grants = Vec::new();
        for declaration in internals {
            let matches: Vec<_> = catalog
                .fields
                .iter()
                .filter(|entry| {
                    entry.path == declaration.path
                        && declaration
                            .kind
                            .as_ref()
                            .is_none_or(|kind| kind == &entry.kind)
                })
                .collect();
            if matches.len() != 1 || matches[0].hash != declaration.hash {
                return Err(format!(
                    "unknown, ambiguous or mismatched internal {}",
                    declaration.path
                ));
            }
            let entry = matches[0];
            let layout = catalog
                .layouts
                .get(entry.receiver)
                .ok_or("unknown receiver layout")?;
            let fields: Vec<_> = layout
                .fields
                .iter()
                .filter(|field| field.name == entry.field)
                .collect();
            if fields.len() != 1 {
                return Err(format!(
                    "unknown or ambiguous receiver field {}",
                    entry.path
                ));
            }
            grants.push((
                entry.receiver,
                Grant {
                    case: fields[0].case,
                    ordinal: fields[0].ordinal,
                },
            ));
        }
        // Only listened points get a plan: a plan is sized by the whole graph.
        let mut plans = std::collections::HashMap::new();
        for &head in listened {
            let point = catalog
                .points
                .iter()
                .find(|point| point.head_call == head)
                .ok_or("listener on an unknown point")?;
            let selected: Vec<_> = grants
                .iter()
                .filter_map(|(receiver, grant)| {
                    (Some(*receiver) == point.receiver).then_some(*grant)
                })
                .collect();
            let mut roots = vec![point.head_call];
            roots.extend(point.tail_call);
            plans.insert(
                head,
                Arc::new(Plan::build(
                    &catalog.layouts,
                    point.receiver,
                    &selected,
                    &roots,
                )?),
            );
        }
        Ok(Self { plans })
    }
    /// Return an owned plan for the point's head call layout, as named in its staged plan.
    pub fn plan(&self, head_call: usize) -> Option<Arc<Plan>> {
        self.plans.get(&head_call).cloned()
    }
}
