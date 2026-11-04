//! The hook catalog as typed data: `native-hooks.json` and the fragments beside it, read once.
//!
//! The loader here is the only code that touches the catalog's JSON. Every generated output
//! (the `Hooks` dispatch, the Host traits, the game's entry points, the rustdoc, the mixin hook
//! names) reads these types.

use serde::Deserialize;
use std::{collections::HashSet, fmt, fs, path::Path};

/// Which side of the connection a hook or helper runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Server,
    Client,
    Both,
}

/// What a hook's listeners may do with its event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Contract {
    /// Told about a moment; nothing comes back.
    Listen,
    /// May veto it; the first listener to cancel stops the rest.
    Cancel,
    /// Every listener adds to, changes or removes from one output.
    Collect,
    /// The first listener to answer sets the output.
    Replace,
}

/// The host a hook takes when its receiver does not say which.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostAccess {
    /// The shared read-only `Host`.
    Read,
    /// A level host, for a receiver that is a level but not `GameLevel`.
    Level,
}

/// A hook fired in a load phase, which takes that phase's context instead of a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseContext {
    Registration,
    DataLoad,
}

/// The kind of thing whose Parts a scoped hook calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartKind {
    Block,
    BlockEntity,
    Item,
    Menu,
}

/// A scoped hook: a Part method, called on the Parts attached to what `key` names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartScope {
    pub kind: PartKind,
    /// The hook argument naming the thing whose Parts are called.
    pub key: String,
}

/// The names a hook has in other mod APIs, for the rustdoc and for finding gaps.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counterparts {
    pub neoforge: Vec<String>,
    pub fabric: Vec<String>,
    pub paper: Vec<String>,
}

/// One hook point: a moment the game tells mods about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hook {
    pub name: String,
    /// The rustdoc of its trait method.
    pub doc: String,
    pub side: Side,
    /// The game function that fires it.
    pub target: String,
    pub contract: Contract,
    /// The event field a cancel, collect or replace hook writes; `None` for a listener.
    pub out: Option<String>,
    /// A replace hook whose output also names the mod that answered.
    pub answered_by_mod: bool,
    pub scope: Option<PartScope>,
    pub host: Option<HostAccess>,
    pub context: Option<PhaseContext>,
    pub counterparts: Counterparts,
    /// The contract version an injection's `hook_version` names.
    pub version: u64,
    /// Whether the NeoForge counterpart event is cancellable, written by the
    /// stamper from the counterpart's javadoc; documentation metadata, not the
    /// contract (a hook that listens can still name a cancellable event).
    pub cancellable: Option<bool>,
}

/// A game function mods reach through the `Host` or a handle.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Helper {
    pub name: String,
    pub target: String,
    pub side: Side,
    /// The both-side helper's client implementation, when it differs from `target`.
    pub client_target: Option<String>,
    /// The handle the helper is also a method of.
    pub receiver: Option<String>,
    #[serde(default)]
    pub backend: bool,
}

/// A drawing surface of the game's that drawing hooks get as a trait object (the catalog's
/// `draw_contexts`): the surface's helpers become methods of a trait, implemented by a
/// game struct pairing `Game` with the surface, and a hook argument of the surface's type is
/// the trait object in the hook's event.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DrawContext {
    /// The game's type, as the host implementation names it (`crate::hud::HudBatch`).
    pub surface: String,
    /// The trait mods draw through.
    #[serde(rename = "trait")]
    pub trait_name: String,
    /// What its helpers' names start with, left off the trait's method names.
    pub prefix: String,
    pub doc: String,
}

/// Why the catalog was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
    Read { path: String, error: String },
    Json { path: String, error: String },
    NotAnArray(&'static str),
    NamelessHook,
    DuplicateHook(String),
    DuplicateHelper(String),
    InvalidHook { hook: String, error: String },
    InvalidHelper { helper: String, error: String },
    MissingDoc(String),
    Counterpart { hook: String, key: &'static str },
    InvalidContract(String),
    AnsweredNeedsGlobalReplace(String),
    ScopeNeedsKey(String),
    ClientTargetNeedsBoth(String),
    NoHooks,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => write!(f, "{path}: {error}"),
            Self::Json { path, error } => write!(f, "{path}: {error}"),
            Self::NotAnArray(what) => write!(f, "{what} must be an array"),
            Self::NamelessHook => f.write_str("hook name missing"),
            Self::DuplicateHook(name) => write!(f, "duplicate hook {name}"),
            Self::DuplicateHelper(name) => write!(f, "duplicate helper {name}"),
            Self::InvalidHook { hook, error } => write!(f, "hook {hook}: {error}"),
            Self::InvalidHelper { helper, error } => write!(f, "helper {helper}: {error}"),
            Self::MissingDoc(name) => write!(
                f,
                "hook {name}: doc missing: add a `doc` field, the rustdoc of its trait method"
            ),
            Self::Counterpart { hook, key } => {
                write!(f, "hook {hook}: {key} must be a string or list of strings")
            }
            Self::InvalidContract(name) => write!(f, "invalid contract for {name}"),
            Self::AnsweredNeedsGlobalReplace(name) => {
                write!(f, "{name}: answered_by needs a global replace hook")
            }
            Self::ScopeNeedsKey(name) => write!(f, "{name} needs both scope and key, or neither"),
            Self::ClientTargetNeedsBoth(name) => {
                write!(f, "helper {name}: client_target requires side both")
            }
            Self::NoHooks => f.write_str("no native hooks"),
        }
    }
}

impl std::error::Error for CatalogError {}

impl From<CatalogError> for String {
    fn from(error: CatalogError) -> Self {
        error.to_string()
    }
}

/// A counterpart name or a list of them.
#[derive(Deserialize)]
#[serde(untagged)]
enum Names {
    One(String),
    Many(Vec<String>),
}

fn counterparts(
    hook: &str,
    key: &'static str,
    names: Option<serde_json::Value>,
) -> Result<Vec<String>, CatalogError> {
    let invalid = || CatalogError::Counterpart {
        hook: hook.to_owned(),
        key,
    };
    match names {
        None => Ok(Vec::new()),
        Some(value) => match serde_json::from_value(value).map_err(|_| invalid())? {
            Names::One(name) => Ok(vec![name]),
            Names::Many(names) => Ok(names),
        },
    }
}

/// A hook entry as the JSON spells it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HookEntry {
    #[serde(rename = "name")]
    _name: String,
    doc: Option<String>,
    side: Side,
    target: String,
    mod_api: ModApiEntry,
    version: Option<u64>,
    cancellable: Option<bool>,
    host: Option<HostAccess>,
    context: Option<PhaseContext>,
    neoforge: Option<serde_json::Value>,
    fabric: Option<serde_json::Value>,
    paper: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModApiEntry {
    contract: Contract,
    out: Option<String>,
    scope: Option<PartKind>,
    key: Option<String>,
    answered_by: Option<AnsweredBy>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AnsweredBy {
    Mod,
}

impl Hook {
    fn parse(value: serde_json::Value) -> Result<Self, CatalogError> {
        let name = value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or(CatalogError::NamelessHook)?
            .to_owned();
        let entry: HookEntry =
            serde_json::from_value(value).map_err(|error| CatalogError::InvalidHook {
                hook: name.clone(),
                error: error.to_string(),
            })?;
        let doc = entry
            .doc
            .filter(|doc| !doc.trim().is_empty())
            .ok_or_else(|| CatalogError::MissingDoc(name.clone()))?;
        let api = entry.mod_api;
        let scope = match (api.scope, api.key) {
            (None, None) => None,
            (Some(kind), Some(key)) => Some(PartScope { kind, key }),
            _ => return Err(CatalogError::ScopeNeedsKey(name)),
        };
        if matches!(
            (api.contract, &api.out),
            (Contract::Listen, Some(_))
                | (
                    Contract::Cancel | Contract::Collect | Contract::Replace,
                    None
                )
        ) {
            return Err(CatalogError::InvalidContract(name));
        }
        let answered_by_mod = api.answered_by.is_some();
        if answered_by_mod && (api.contract != Contract::Replace || scope.is_some()) {
            return Err(CatalogError::AnsweredNeedsGlobalReplace(name));
        }
        let counterparts = Counterparts {
            neoforge: counterparts(&name, "neoforge", entry.neoforge)?,
            fabric: counterparts(&name, "fabric", entry.fabric)?,
            paper: counterparts(&name, "paper", entry.paper)?,
        };
        Ok(Self {
            name,
            doc,
            side: entry.side,
            target: entry.target,
            contract: api.contract,
            out: api.out,
            answered_by_mod,
            scope,
            host: entry.host,
            context: entry.context,
            counterparts,
            version: entry.version.unwrap_or(1),
            cancellable: entry.cancellable,
        })
    }
}

impl Helper {
    fn parse(value: serde_json::Value) -> Result<Self, CatalogError> {
        let name = value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let helper: Self =
            serde_json::from_value(value).map_err(|error| CatalogError::InvalidHelper {
                helper: name.clone(),
                error: error.to_string(),
            })?;
        if helper.client_target.is_some() && helper.side != Side::Both {
            return Err(CatalogError::ClientTargetNeedsBoth(name));
        }
        Ok(helper)
    }
}

/// Everything the generator reads from the catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    /// In file order: `native-hooks.json`, then each fragment by file name.
    pub hooks: Vec<Hook>,
    pub helpers: Vec<Helper>,
    pub draw_contexts: Vec<DrawContext>,
}

fn read_json(path: &Path) -> Result<serde_json::Value, CatalogError> {
    let shown = path.display().to_string();
    let text = fs::read_to_string(path).map_err(|e| CatalogError::Read {
        path: shown.clone(),
        error: e.to_string(),
    })?;
    serde_json::from_str(&text).map_err(|e| CatalogError::Json {
        path: shown,
        error: e.to_string(),
    })
}

/// The array under `key`, taken out of `value`; `what` names it for the error.
fn entries(
    value: &mut serde_json::Value,
    key: &str,
    what: &'static str,
) -> Result<Vec<serde_json::Value>, CatalogError> {
    match value.get_mut(key) {
        Some(serde_json::Value::Array(entries)) => Ok(std::mem::take(entries)),
        _ => Err(CatalogError::NotAnArray(what)),
    }
}

impl Catalog {
    /// The catalog of the checkout at `root`: `native-hooks.json` and `tools/native-hook-fragments`.
    pub fn load(root: &Path) -> Result<Self, CatalogError> {
        Self::assemble(
            read_json(&root.join("native-hooks.json"))?,
            &root.join("tools/native-hook-fragments"),
        )
    }

    /// The catalog of `base` (the contents of `native-hooks.json`) and the fragments in `fragments`.
    pub fn assemble(mut base: serde_json::Value, fragments: &Path) -> Result<Self, CatalogError> {
        let draw_contexts = match base.get("draw_contexts") {
            None => Vec::new(),
            Some(value) => {
                serde_json::from_value(value.clone()).map_err(|e| CatalogError::Json {
                    path: "catalog draw_contexts".into(),
                    error: e.to_string(),
                })?
            }
        };
        let mut raw_hooks = entries(&mut base, "hooks", "catalog hooks")?;
        let mut raw_helpers = match base.get("helpers") {
            None => Vec::new(),
            Some(_) => entries(&mut base, "helpers", "catalog helpers")?,
        };
        let mut paths = fs::read_dir(fragments)
            .map_err(|e| CatalogError::Read {
                path: fragments.display().to_string(),
                error: e.to_string(),
            })?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|e| CatalogError::Read {
                        path: fragments.display().to_string(),
                        error: e.to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        for path in paths
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        {
            let mut fragment = read_json(&path)?;
            raw_hooks.extend(entries(&mut fragment, "hooks", "fragment hooks")?);
            if fragment.get("helpers").is_some() {
                raw_helpers.extend(entries(&mut fragment, "helpers", "fragment helpers")?);
            }
        }
        let mut seen = HashSet::new();
        let hooks = raw_hooks
            .into_iter()
            .map(|raw| {
                let hook = Hook::parse(raw)?;
                if seen.insert(hook.name.clone()) {
                    Ok(hook)
                } else {
                    Err(CatalogError::DuplicateHook(hook.name))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = HashSet::new();
        let helpers = raw_helpers
            .into_iter()
            .map(|raw| {
                let helper = Helper::parse(raw)?;
                if seen.insert(helper.name.clone()) {
                    Ok(helper)
                } else {
                    Err(CatalogError::DuplicateHelper(helper.name))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            hooks,
            helpers,
            draw_contexts,
        })
    }
}

#[cfg(test)]
impl Hook {
    /// A global listener with no counterparts, for tests of what reads a hook.
    pub(crate) fn sample(target: &str, side: Side) -> Self {
        Self {
            name: "test".into(),
            doc: "Test.".into(),
            side,
            target: target.into(),
            contract: Contract::Listen,
            out: None,
            answered_by_mod: false,
            scope: None,
            host: None,
            context: None,
            counterparts: Counterparts::default(),
            version: 1,
            cancellable: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hook(name: &str, extra: serde_json::Value) -> serde_json::Value {
        let mut hook = json!({
            "name": name, "doc": "Doc.", "side": "server",
            "target": "horizon_client::game::native_hooks::Game<'_>::native_x",
            "mod_api": {"contract": "listen"}
        });
        hook.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        hook
    }

    fn empty_fragments(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("catalog-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_shipped_catalog_loads_with_typed_contracts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog = Catalog::load(&root).unwrap();
        let find = |name: &str| catalog.hooks.iter().find(|h| h.name == name).unwrap();
        let scoped = find("pre_remove_side_effects");
        assert_eq!(scoped.scope.as_ref().unwrap().kind, PartKind::BlockEntity);
        assert_eq!(find("recipes_received").contract, Contract::Listen);
        assert!(
            catalog
                .hooks
                .iter()
                .any(|h| h.contract == Contract::Replace)
        );
        assert!(!catalog.draw_contexts.is_empty());
    }

    #[test]
    fn a_fragment_repeating_a_hook_is_refused() {
        let dir = empty_fragments("duplicate");
        std::fs::write(dir.join("empty.json"), r#"{"hooks":[]}"#).unwrap();
        std::fs::write(
            dir.join("duplicate.json"),
            json!({"hooks": [hook("same", json!({}))]}).to_string(),
        )
        .unwrap();
        let base = json!({"hooks": [hook("same", json!({}))]});
        assert_eq!(
            Catalog::assemble(base.clone(), &dir).unwrap_err(),
            CatalogError::DuplicateHook("same".into())
        );
        std::fs::remove_file(dir.join("duplicate.json")).unwrap();
        assert_eq!(Catalog::assemble(base, &dir).unwrap().hooks.len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_hook_without_a_doc_is_refused() {
        for doc in [json!(null), json!("  ")] {
            assert_eq!(
                Hook::parse(hook("bare", json!({"doc": doc})))
                    .unwrap_err()
                    .to_string(),
                "hook bare: doc missing: add a `doc` field, the rustdoc of its trait method"
            );
        }
    }

    #[test]
    fn counterparts_accept_strings_lists_and_absence() {
        let one = Hook::parse(hook(
            "one",
            json!({"fabric": "Event.ONE", "neoforge": "OneEvent"}),
        ))
        .unwrap();
        assert_eq!(one.counterparts.fabric, ["Event.ONE"]);
        assert_eq!(one.counterparts.neoforge, ["OneEvent"]);
        let two =
            Hook::parse(hook("two", json!({"fabric": ["Event.TWO", "Event.THREE"]}))).unwrap();
        assert_eq!(two.counterparts.fabric.len(), 2);
        assert_eq!(
            Hook::parse(hook("none", json!({}))).unwrap().counterparts,
            Counterparts::default()
        );
        for key in ["fabric", "neoforge", "paper"] {
            assert_eq!(
                Hook::parse(hook("bad", json!({key: ["ok", 4]})))
                    .unwrap_err()
                    .to_string(),
                format!("hook bad: {key} must be a string or list of strings")
            );
        }
    }

    #[test]
    fn a_hook_needs_a_side_a_target_and_a_contract() {
        for side in [json!(null), json!("invalid")] {
            assert!(Hook::parse(hook("bad", json!({"side": side}))).is_err());
        }
        for field in ["side", "target", "mod_api"] {
            let mut value = hook("bad", json!({}));
            value.as_object_mut().unwrap().remove(field);
            assert!(
                Hook::parse(value)
                    .unwrap_err()
                    .to_string()
                    .contains(&format!("missing field `{field}`"))
            );
        }
        assert_eq!(
            Hook::parse(json!({})).unwrap_err(),
            CatalogError::NamelessHook
        );
    }

    #[test]
    fn contracts_and_outputs_go_together() {
        let parse = |contract: &str, out: Option<&str>| {
            Hook::parse(hook(
                "c",
                json!({"mod_api": {"contract": contract, "out": out}}),
            ))
        };
        assert_eq!(parse("listen", None).unwrap().contract, Contract::Listen);
        for contract in ["cancel", "collect", "replace"] {
            assert_eq!(
                parse(contract, Some("o")).unwrap().out.as_deref(),
                Some("o")
            );
            assert_eq!(
                parse(contract, None).unwrap_err(),
                CatalogError::InvalidContract("c".into())
            );
        }
        assert_eq!(
            parse("listen", Some("o")).unwrap_err(),
            CatalogError::InvalidContract("c".into())
        );
        assert!(parse("unknown", None).is_err());
    }

    #[test]
    fn a_scope_and_its_key_go_together() {
        let scoped = |api: serde_json::Value| Hook::parse(hook("s", json!({"mod_api": api})));
        let hook_scope = scoped(json!({"contract": "listen", "scope": "item", "key": "stack"}))
            .unwrap()
            .scope
            .unwrap();
        assert_eq!(
            (hook_scope.kind, hook_scope.key.as_str()),
            (PartKind::Item, "stack")
        );
        for api in [
            json!({"contract": "listen", "scope": "item"}),
            json!({"contract": "listen", "key": "stack"}),
        ] {
            assert_eq!(
                scoped(api).unwrap_err(),
                CatalogError::ScopeNeedsKey("s".into())
            );
        }
        assert!(scoped(json!({"contract": "listen", "scope": "nothing", "key": "k"})).is_err());
    }

    #[test]
    fn answered_by_needs_a_global_replace_hook() {
        let answered = |api: serde_json::Value| Hook::parse(hook("a", json!({"mod_api": api})));
        assert!(
            answered(json!({"contract": "replace", "out": "o", "answered_by": "mod"}))
                .unwrap()
                .answered_by_mod
        );
        for api in [
            json!({"contract": "cancel", "out": "o", "answered_by": "mod"}),
            json!({"contract": "replace", "out": "o", "answered_by": "mod", "scope": "item", "key": "k"}),
        ] {
            assert_eq!(
                answered(api).unwrap_err(),
                CatalogError::AnsweredNeedsGlobalReplace("a".into())
            );
        }
    }

    #[test]
    fn a_client_target_needs_a_both_side_helper() {
        let helper = |side: &str| {
            Helper::parse(json!({"name": "h", "target": "t", "side": side, "client_target": "c"}))
        };
        assert_eq!(helper("both").unwrap().client_target.as_deref(), Some("c"));
        assert_eq!(
            helper("server").unwrap_err(),
            CatalogError::ClientTargetNeedsBoth("h".into())
        );
        assert!(Helper::parse(json!({"name": "h", "target": "t"})).is_err());
    }
}
