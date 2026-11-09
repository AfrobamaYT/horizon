//! A mod's own injection points (#2339), as the `points.json` its build writes lists them, and the
//! routing of an addon's `[[inject]]` to them. Mod points mirror the game's: the same declarations,
//! `api` and `internals` levels and refusals, plus a signature hash an addon pins, checked when it
//! loads: a library rebuilt with a changed function refuses the addon instead of running it
//! against another signature. The native launcher and the wasm loader route through this one
//! function.
use super::{Entry, Inject, InjectError, NoSlotReason, Place, Route, SlotKind, Target};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The file a mod's build writes its points to, at the package root.
pub const FILE: &str = "points.json";

/// A mod crate's points, as its build lists them.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// The crate's library name: an injection's `at` starts with it.
    #[serde(rename = "crate")]
    pub krate: String,
    /// The slot table module at the crate's root.
    pub table: String,
    /// The functions `#[mod_api::no_point]` opted out, by path.
    #[serde(default)]
    pub opted_out: Vec<String>,
    pub points: Vec<Point>,
}

/// One mod point.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Point {
    /// Its place's name in the table: the head; the return adds `_return`.
    pub name: String,
    /// The function's path, as an injection's `at` names it.
    pub target: String,
    /// What an addon's declaration pins.
    pub hash: String,
    /// The signature the hash covers.
    pub signature: String,
    /// Whether it has an internals form.
    pub raw: bool,
    /// Whether its API listeners have a return place.
    pub at_return: bool,
    /// Why its API listeners are refused, naming the parameter and type.
    #[serde(default)]
    pub api_refusal: Option<String>,
    /// What its listeners may do with the return: `cancel`, `result` or `none`.
    pub returns: String,
    /// Its API listeners' host: `none` for every mod point.
    pub host: String,
    /// The function's return type as written.
    pub return_ty: String,
    /// Its internals head and return listener types, by path from the crate root.
    #[serde(default)]
    pub listener: Option<String>,
    #[serde(default)]
    pub return_listener: Option<String>,
}

/// An installed library whose points an addon may target.
pub struct Library<'a> {
    pub id: &'a str,
    pub version: &'a str,
    /// Built for the same target as the addon: the same binary on native, or a wasm library
    /// whose dependents link into its dependency group.
    pub same_target: bool,
    pub catalog: &'a Catalog,
}

impl Catalog {
    /// Whether `at` names a function of this crate.
    pub fn owns(&self, at: &str) -> bool {
        at.split("::").next() == Some(self.krate.as_str())
    }
}

/// Where an injection listens in its library's table.
#[derive(Debug, PartialEq, Eq)]
pub struct Routed<'a> {
    pub point: &'a Point,
    /// The place's name: the point's, or its `_return`.
    pub place: String,
    pub access: super::Access,
}

/// Why an addon's injection at a mod point doesn't load.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A routing refusal shared with the game's points (missing, no slot, no API record).
    Inject(InjectError),
    /// The library's function changed since the addon pinned it.
    Changed {
        id: String,
        library: String,
        version: String,
        at: String,
        pinned: String,
        installed: String,
        signature: String,
    },
    /// The function exists but its author opted it out with `#[mod_api::no_point]`.
    OptedOut { id: String, at: String },
    /// `internals` reaches the library's own types, so it needs the library's target.
    CrossTargetInternals { id: String, library: String },
    /// An `api` record crosses targets only with a `Wire` form for every parameter.
    CrossTargetNoWire {
        id: String,
        library: String,
        reason: String,
    },
    /// Mod points don't cross targets yet: mixed native and wasm dependencies are refused.
    CrossTarget { id: String, library: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inject(error) => error.fmt(f),
            Self::Changed {
                id,
                library,
                version,
                at,
                pinned,
                installed,
                signature,
            } => write!(
                f,
                "E_INJECT_CHANGED: {id}: {library} {version} changed {at}: pinned {pinned}, \
                 installed {installed} ({signature}); update the declaration with `cargo \
                 horizon-mod inject update` and rebuild against it"
            ),
            Self::OptedOut { id, at } => write!(
                f,
                "E_INJECT_MISSING: {id}: {at} has no point (opted out with #[mod_api::no_point])"
            ),
            Self::CrossTargetInternals { id, library } => write!(
                f,
                "{id}: internals access to {library} needs the same target: the same binary on \
                 native, the same dependency group on wasm"
            ),
            Self::CrossTargetNoWire {
                id,
                library,
                reason,
            } => write!(
                f,
                "{id}: an api listener at {library} on the other target crosses through the \
                 game, and {reason}: add #[derive(mod_api::Wire)] to it"
            ),
            Self::CrossTarget { id, library } => write!(
                f,
                "{id}: {library} is on the other target: mixed native and wasm dependencies \
                 aren't supported yet"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Routes `inject`, an `[[inject]]` of mod `mod_id` whose `at` the `library` owns, to its place, or
/// refuses it with the reason a player sees.
pub fn route<'a>(
    mod_id: &str,
    inject: &Inject,
    library: &Library<'a>,
) -> Result<Routed<'a>, Refusal> {
    let id = format!("{mod_id}:{}", inject.id);
    let plain = |path: &str| path.replace(' ', "");
    let catalog = library.catalog;
    let Some(point) = catalog
        .points
        .iter()
        .find(|point| plain(&point.target) == plain(&inject.at))
    else {
        if catalog
            .opted_out
            .iter()
            .any(|path| plain(path) == plain(&inject.at))
        {
            return Err(Refusal::OptedOut {
                id,
                at: inject.at.clone(),
            });
        }
        return Err(Refusal::Inject(InjectError::Missing {
            id,
            at: inject.at.clone(),
        }));
    };
    if point.hash != inject.hash {
        return Err(Refusal::Changed {
            id,
            library: library.id.to_owned(),
            version: library.version.to_owned(),
            at: inject.at.clone(),
            pinned: inject.hash.clone(),
            installed: point.hash.clone(),
            signature: point.signature.clone(),
        });
    }
    if !library.same_target {
        let library = library.id.to_owned();
        return Err(match (inject.access, &point.api_refusal) {
            (super::Access::Internals, _) => Refusal::CrossTargetInternals { id, library },
            (super::Access::Api, Some(reason)) => Refusal::CrossTargetNoWire {
                id,
                library,
                reason: reason.clone(),
            },
            (super::Access::Api, None) => Refusal::CrossTarget { id, library },
        });
    }
    let api = inject.access == super::Access::Api;
    if let (true, Some(why)) = (api, &point.api_refusal) {
        return Err(Refusal::Inject(InjectError::NoSlot {
            id,
            reason: NoSlotReason::NoApiRecord,
            declaration: format!("{}: {why}", inject.at),
        }));
    }
    let mut slots = Vec::new();
    if api || point.listener.is_some() {
        slots.push(SlotKind::Head);
    }
    if (api && point.at_return) || (!api && point.return_listener.is_some()) {
        slots.push(SlotKind::Return);
    }
    let place = Place {
        at: inject.at.clone(),
        hash: point.hash.clone(),
        slots,
        calls: Vec::new(),
        native_only: false,
    };
    // A mod point has head and return places only: call-site kinds have none.
    if inject.kind.needs_call() {
        return Err(Refusal::Inject(InjectError::NoSlot {
            id,
            reason: if api {
                NoSlotReason::ApiSitesPending
            } else {
                NoSlotReason::NotListed
            },
            declaration: format!("{}: a mod point has head and return places only", inject.at),
        }));
    }
    match super::route(
        Entry {
            inject,
            target: Target::Native,
            mod_id,
            resolved_call: None,
        },
        std::slice::from_ref(&place),
    )
    .map_err(Refusal::Inject)?
    {
        Route::Slot(SlotKind::Head) => Ok(Routed {
            point,
            place: point.name.clone(),
            access: inject.access,
        }),
        Route::Slot(_) => Ok(Routed {
            point,
            place: format!("{}_return", point.name),
            access: inject.access,
        }),
        Route::Source(reason) => Err(Refusal::Inject(InjectError::NoSlot {
            id,
            reason,
            declaration: inject.at.clone(),
        })),
    }
}

#[cfg(test)]
#[path = "mod_points_tests.rs"]
mod tests;
