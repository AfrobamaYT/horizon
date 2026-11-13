//! What a mod uses of a surface, and whether it fits another build's surface (design §10.3).
//!
//! [`uses`] scans a mod's Rust source ([`crate::scan`]) and lists the surface records it names,
//! then every record their signatures reference, transitively ([`Surface::closure`]). A record
//! counts when its name occurs as an identifier in the source or the manifest names it, with two
//! narrowings:
//! - a member (a trait method, an inherent item, an associated item, a field, a variant, a hook's
//!   argument) counts only when its owner trait, type or hook is in the closure of what counts:
//!   named in the source, or reached from a hook, method or function the mod uses;
//! - a name written after `X::`, where `X` is something with a member of that name, is that
//!   member only (`TextHover::Item` is the variant, not a type `Item`).
//!
//! For each struct the source names it also records how the mod builds it: [`Built::Full`] when a
//! literal or pattern `Name { … }` (or `Name(…)` for a tuple struct) lists its fields with no `..`,
//! [`Built::Base`] when every one ends in `..`, [`Built::None`] otherwise. A full literal breaks
//! when the struct gains a field; the others do not. When the scan cannot tell (an unclosed
//! literal), it says full: a refusal is safe, a wrong "fits" is not.
//!
//! [`fits`] compares such a list with a target build's surface. The mod fits when every entry
//! exists there with an equal hash, every type keeps the trait impls it had, every struct built
//! with a full literal keeps its field set, and every hook whose version the manifest names keeps
//! that version. The per-member hashes make this exact about additions: a new `Host` method, a new
//! hook, a new hook argument, a new field on a struct the mod does not build in full, or a new
//! trait impl changes nothing for the mod.
//!
//! Whether a mod is exact by nature (a `[native.*]` table, internals) is the caller's to decide
//! from its manifest before asking: this crate does not read manifests.
//!
//! A [`UsesTable`] is the list a package carries: schema, the build it was made for, and its
//! entries. [`UsesTable::read`] refuses an unknown schema, malformed JSON, repeated ids and
//! documents over [`MAX_USES_BYTES`] with a typed [`UsesError`].
use crate::scan::{self, Token};
use crate::{Surface, Type, TypeKind, leaf};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What a mod's manifest says that the list depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManifestFacts {
    /// Capability keys and hooks it names.
    pub names: Vec<String>,
    /// Hooks it names a version of, with that version.
    pub hook_versions: BTreeMap<String, u64>,
}

/// Why an entry is in the list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// Its name occurs at this line (1-based) of this source file: the first such place, by
    /// file path then line.
    Source { file: String, line: usize },
    /// The manifest names it: a capability, or a hook by name.
    Manifest,
    /// A signature of this entry references it, directly or through others.
    Via(String),
}

/// How a mod builds a struct it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Built {
    /// Never with a literal or pattern.
    None,
    /// Only with literals and patterns that end in `..`.
    Base,
    /// With at least one literal or pattern that lists its fields with no `..`.
    Full,
}

/// One record a mod uses, as the table carries it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    /// Its hash at the made-for build.
    pub hash: String,
    /// A type's trait impls at the made-for build: the target must keep each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impls: Vec<String>,
    /// For a struct the source names: how the mod builds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built: Option<Built>,
    /// With [`Built::Full`]: the struct's `fields_hash` at the made-for build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields_hash: Option<String>,
    /// With [`Built::Full`] or [`Built::Base`]: `file:line` of the first such literal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    /// For a hook whose version the manifest names: that version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

impl Entry {
    /// What `check --uses` tells an author about how the mod builds this struct, when it builds
    /// it with a literal.
    pub fn building_note(&self) -> Option<String> {
        let name = self.id.strip_prefix("type:")?;
        let at = self.built_at.as_deref()?;
        match self.built? {
            Built::None => None,
            Built::Base => Some(format!(
                "`{name}`: built with `..` ({at}), so a new field in a later game build will not break this mod."
            )),
            Built::Full if self.impls.iter().any(|i| i == "Default") => Some(format!(
                "`{name}`: built field by field ({at}). A new field in a later game build will break this line; end the literal with `..Default::default()` to stay compatible."
            )),
            Built::Full => Some(format!(
                "`{name}`: built field by field ({at}). A new field in a later game build will break this line."
            )),
        }
    }
}

/// An entry and why it is there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Used {
    pub entry: Entry,
    pub origin: Origin,
}

/// What a mod uses of one surface, sorted by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uses {
    pub items: Vec<Used>,
}

impl Uses {
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.items.iter().map(|used| &used.entry)
    }

    /// The table a package carries for a mod made for `made_for` (a build id).
    pub fn table(&self, made_for: impl Into<String>) -> UsesTable {
        UsesTable {
            schema: USES_SCHEMA,
            made_for: made_for.into(),
            entries: self.entries().cloned().collect(),
        }
    }
}

/// The owner of a member id: a `trait:`, `type:` or `hook:` id (for an inherent item, whichever
/// the surface has); `None` for a record that is not a member.
fn owner(id: &str, ids: &BTreeMap<&str, &str>) -> Option<String> {
    let (kind, rest) = id.split_once(':')?;
    let owner_path = |rest: &str| rest.rsplit_once("::").map(|(owner, _)| owner.to_owned());
    match kind {
        "method" | "assoc" => owner_path(rest).map(|path| format!("trait:{path}")),
        "handle" => owner_path(rest).map(|path| {
            let as_type = format!("type:{path}");
            if ids.contains_key(as_type.as_str()) {
                as_type
            } else {
                format!("trait:{path}")
            }
        }),
        "type" | "hook" => rest
            .rsplit_once('.')
            .map(|(owner, _)| format!("{kind}:{owner}")),
        _ => None,
    }
}

/// A place in the mod's source.
type Place = (String, usize);

/// What one source file says about a struct-like type.
#[derive(Default)]
struct Building {
    full: Option<Place>,
    base: Option<Place>,
}

/// The index of the token that closes the bracket opened at `open`, if it closes.
fn closing(tokens: &[(Token<'_>, usize)], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, (token, _)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Punct(b'(' | b'[' | b'{') => depth += 1,
            Token::Punct(b')' | b']' | b'}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether the literal or pattern whose bracket opens at `open` ends its field list with `..`
/// (a base expression or a rest pattern). Unclosed: no, so the caller says full.
fn has_rest(tokens: &[(Token<'_>, usize)], open: usize) -> bool {
    let Some(close) = closing(tokens, open) else {
        return false;
    };
    let mut depth = 0usize;
    let mut previous = Token::Punct(b'{');
    for index in open + 1..close {
        let token = tokens[index].0;
        match token {
            Token::Punct(b'(' | b'[' | b'{') => depth += 1,
            Token::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            Token::Punct(b'.')
                if depth == 0
                    && tokens[index + 1].0 == Token::Punct(b'.')
                    && matches!(previous, Token::Punct(b'{' | b'(' | b','))
                    && index > open =>
            {
                return true;
            }
            _ => {}
        }
        if depth == 0 {
            previous = token;
        }
    }
    false
}

/// Whether the name at `at` stands where a type does, not a value: after `->`, `impl`, `for`,
/// `dyn` or a definition keyword, path segments before it skipped.
fn in_type_position(tokens: &[(Token<'_>, usize)], mut at: usize) -> bool {
    while at >= 3
        && tokens[at - 1].0 == Token::Punct(b':')
        && tokens[at - 2].0 == Token::Punct(b':')
        && matches!(tokens[at - 3].0, Token::Ident(_))
    {
        at -= 3;
    }
    match at.checked_sub(1).map(|i| tokens[i].0) {
        Some(Token::Ident(
            "impl" | "for" | "dyn" | "struct" | "enum" | "union" | "trait" | "type",
        )) => true,
        Some(Token::Punct(b'>')) => at >= 2 && tokens[at - 2].0 == Token::Punct(b'-'),
        _ => false,
    }
}

/// The records `sources` (each a path and its Rust text) and `manifest` bring in, with
/// everything their signatures reference.
pub fn uses<P: AsRef<str>, T: AsRef<str>>(
    sources: impl IntoIterator<Item = (P, T)>,
    manifest: &ManifestFacts,
    surface: &Surface,
) -> Uses {
    let hashes: BTreeMap<&str, &str> = surface.entries().collect();
    let types: BTreeMap<&str, &Type> = surface.types.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, id) in surface.identifiers() {
        by_name.entry(name).or_default().push(id);
    }
    // The leaves of the owners of each member name: `Item` → {`TextHover`}.
    let owners: BTreeMap<&str, String> = hashes
        .keys()
        .filter_map(|id| owner(id, &hashes).map(|owner| (*id, owner)))
        .collect();
    let struct_like = |id: &str| {
        types.get(id).is_some_and(|t| {
            matches!(
                t.contract.kind,
                TypeKind::Struct | TypeKind::TupleStruct | TypeKind::Union
            )
        })
    };

    // The records named directly, each with the first place that names it, and how each named
    // struct is built.
    let mut direct: BTreeMap<&str, Origin> = BTreeMap::new();
    let mut building: BTreeMap<&str, Building> = BTreeMap::new();
    for (path, text) in sources {
        let path = path.as_ref();
        let tokens: Vec<(Token<'_>, usize)> = scan::tokens(text.as_ref()).collect();
        for (at, (token, line)) in tokens.iter().enumerate() {
            let Token::Ident(name) = token else {
                continue;
            };
            let Some(ids) = by_name.get(name) else {
                continue;
            };
            let qualifier = match at.checked_sub(3).map(|i| &tokens[i..at]) {
                Some(
                    [
                        (Token::Ident(q), _),
                        (Token::Punct(b':'), _),
                        (Token::Punct(b':'), _),
                    ],
                ) => Some(*q),
                _ => None,
            };
            let members: Vec<&str> = qualifier
                .map(|q| {
                    ids.iter()
                        .copied()
                        .filter(|id| owners.get(id).is_some_and(|o| leaf(o) == q))
                        .collect()
                })
                .unwrap_or_default();
            let named = if members.is_empty() {
                ids.clone()
            } else {
                members
            };
            let here = Origin::Source {
                file: path.to_owned(),
                line: *line,
            };
            for id in &named {
                direct
                    .entry(id)
                    .and_modify(|origin| *origin = here.clone().min(origin.clone()))
                    .or_insert_with(|| here.clone());
                if !struct_like(id) {
                    continue;
                }
                let tuple = types[id].contract.kind == TypeKind::TupleStruct;
                let opens = tokens.get(at + 1).is_some_and(|(next, _)| {
                    *next == Token::Punct(if tuple { b'(' } else { b'{' })
                });
                if !opens || in_type_position(&tokens, at) {
                    continue;
                }
                let place = (path.to_owned(), *line);
                let entry = building.entry(id).or_default();
                let slot = if has_rest(&tokens, at + 1) {
                    &mut entry.base
                } else {
                    &mut entry.full
                };
                if slot.as_ref().is_none_or(|old| place < *old) {
                    *slot = Some(place);
                }
            }
        }
    }
    for name in &manifest.names {
        for id in [format!("hook:{name}"), format!("capability:{name}")] {
            if let Some((id, _)) = hashes.get_key_value(id.as_str()) {
                direct.entry(id).or_insert(Origin::Manifest);
            }
        }
    }

    // A member counts once its owner is in the closure of what counts.
    let mut counted: BTreeSet<&str> = direct
        .keys()
        .filter(|id| !owners.contains_key(*id))
        .copied()
        .collect();
    loop {
        let reached = surface.closure(counted.iter().copied());
        let more: Vec<&str> = direct
            .keys()
            .filter(|id| {
                !counted.contains(*id) && owners.get(*id).is_some_and(|o| reached.contains(o))
            })
            .copied()
            .collect();
        if more.is_empty() {
            break;
        }
        counted.extend(more);
    }

    // The closure, each reached record naming the counted record it was first reached from.
    let refs = surface.refs();
    let mut origins: BTreeMap<&str, Origin> =
        counted.iter().map(|id| (*id, direct[id].clone())).collect();
    for start in &counted {
        let mut pending = vec![*start];
        while let Some(id) = pending.pop() {
            for next in refs.get(id).into_iter().flatten() {
                if hashes.contains_key(next) && !origins.contains_key(next) {
                    origins.insert(next, Origin::Via((*start).to_owned()));
                    pending.push(next);
                }
            }
        }
    }
    let place = |(file, line): &Place| format!("{file}:{line}");
    let hook_versions = &manifest.hook_versions;
    Uses {
        items: origins
            .into_iter()
            .map(|(id, origin)| {
                let record = types.get(id);
                let built = struct_like(id).then(|| match building.get(id) {
                    Some(Building { full: Some(_), .. }) => Built::Full,
                    Some(Building { base: Some(_), .. }) => Built::Base,
                    _ => Built::None,
                });
                let built_at = building
                    .get(id)
                    .and_then(|b| b.full.as_ref().or(b.base.as_ref()))
                    .map(place);
                Used {
                    entry: Entry {
                        id: id.to_owned(),
                        hash: hashes[id].to_owned(),
                        impls: record.map(|t| t.impls.clone()).unwrap_or_default(),
                        fields_hash: (built == Some(Built::Full))
                            .then(|| record.and_then(|t| t.fields_hash.clone()))
                            .flatten(),
                        built,
                        built_at,
                        version: id
                            .strip_prefix("hook:")
                            .and_then(|name| hook_versions.get(name).copied()),
                    },
                    origin,
                }
            })
            .collect(),
    }
}

/// How a used item differs in the target build.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// The target has no record with its id.
    Removed,
    /// The target's record has another hash.
    Changed,
    /// The target's type no longer implements this trait.
    LostImpl(String),
    /// The struct's field set changed, and the mod builds it with a full literal at `at`: the
    /// target's fields the mod's list does not name (none when only order or a position
    /// changed).
    Fields { gained: Vec<String>, at: String },
    /// The manifest names version `named` of the hook; the target's is `now`.
    Version { named: u64, now: u64 },
}

/// A used item that is not the same in the target build.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Misfit {
    pub id: String,
    pub change: Change,
}

impl std::fmt::Display for Misfit {
    /// `the hook `player_logged_in` (changed)`, `the method `ServerHost::give_item` (removed)`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let item = describe(&self.id);
        match &self.change {
            Change::Removed => write!(f, "{item} (removed)"),
            Change::Changed => write!(f, "{item} (changed)"),
            Change::LostImpl(name) => write!(f, "{item} no longer implements `{name}`"),
            Change::Fields { gained, at } => match &gained[..] {
                [] => write!(
                    f,
                    "the fields of {item} changed, and `{at}` builds it field by field"
                ),
                [one] => write!(
                    f,
                    "{item} gained the field `{one}`, and `{at}` builds it field by field"
                ),
                more => write!(
                    f,
                    "{item} gained the fields {}, and `{at}` builds it field by field",
                    more.iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
            Change::Version { named, now } => {
                write!(
                    f,
                    "{item} is now version {now}; this mod names version {named}"
                )
            }
        }
    }
}

/// An item as people read it: its kind and the name a mod writes for it.
fn describe(id: &str) -> String {
    let Some((kind, rest)) = id.split_once(':') else {
        return format!("`{id}`");
    };
    let (word, name) = match kind {
        "hook" => match rest.split_once('.') {
            Some((hook, argument)) => ("argument", format!("{hook}.{argument}")),
            None => ("hook", rest.to_owned()),
        },
        "trait" => ("trait", rest.to_owned()),
        "method" | "handle" => ("method", rest.to_owned()),
        "assoc" => ("associated item", rest.to_owned()),
        "fn" => ("function", rest.to_owned()),
        "macro" => ("macro", rest.to_owned()),
        "part" => ("Part kind", rest.to_owned()),
        "capability" => ("capability", rest.to_owned()),
        "type" => match rest.rsplit_once('.') {
            // By Rust's naming: a variant is capitalised, a field is not (or is a position).
            Some((owner, member)) if member.starts_with(|c: char| c.is_ascii_uppercase()) => {
                ("variant", format!("{owner}::{member}"))
            }
            Some((owner, member)) => ("field", format!("{owner}.{member}")),
            None => ("type", rest.to_owned()),
        },
        _ => return format!("`{id}`"),
    };
    format!("the {word} `{name}`")
}

/// Whether a mod's list fits a target build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fit {
    Fits,
    /// What is removed or changed in the target, sorted by id.
    DoesNotFit(Vec<Misfit>),
}

/// Whether every entry exists in `target` with an equal hash, every type keeps its impls, every
/// struct built in full keeps its field set and every named hook version is unchanged.
pub fn fits<'a>(entries: impl IntoIterator<Item = &'a Entry>, target: &Surface) -> Fit {
    let hashes: BTreeMap<&str, &str> = target.entries().collect();
    let types: BTreeMap<&str, &Type> = target.types.iter().map(|t| (t.id.as_str(), t)).collect();
    let versions: BTreeMap<&str, u64> = target
        .hooks
        .iter()
        .map(|h| (h.id.as_str(), h.contract.version))
        .collect();
    let entries: Vec<&Entry> = entries.into_iter().collect();
    let listed: BTreeSet<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    let mut misfits = Vec::new();
    for entry in &entries {
        let id = entry.id.as_str();
        let mut misfit = |change| {
            misfits.push(Misfit {
                id: entry.id.clone(),
                change,
            })
        };
        let Some(hash) = hashes.get(id) else {
            misfit(Change::Removed);
            continue;
        };
        if *hash != entry.hash {
            misfit(Change::Changed);
        }
        if let Some(record) = types.get(id) {
            for name in &entry.impls {
                if !record.impls.contains(name) {
                    misfit(Change::LostImpl(name.clone()));
                }
            }
            if entry.built == Some(Built::Full) && entry.fields_hash != record.fields_hash {
                let gained = record
                    .fields
                    .iter()
                    .filter(|f| !listed.contains(f.id.as_str()))
                    .map(|f| leaf(&f.id).to_owned())
                    .collect();
                misfit(Change::Fields {
                    gained,
                    at: entry.built_at.clone().unwrap_or_default(),
                });
            }
        }
        if let (Some(named), Some(now)) = (entry.version, versions.get(id))
            && named != *now
        {
            misfit(Change::Version { named, now: *now });
        }
    }
    misfits.sort();
    misfits.dedup();
    if misfits.is_empty() {
        Fit::Fits
    } else {
        Fit::DoesNotFit(misfits)
    }
}

/// The schema [`UsesTable`] writes and reads.
pub const USES_SCHEMA: u32 = 1;

/// The largest table [`UsesTable::read`] reads, in bytes.
pub const MAX_USES_BYTES: usize = 4 << 20;

/// The list a package carries: what the mod uses, with each item's hash at the build it was made
/// for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsesTable {
    pub schema: u32,
    /// The build id (commit) of the surface the hashes come from.
    pub made_for: String,
    pub entries: Vec<Entry>,
}

/// Why a table was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsesError {
    /// Larger than [`MAX_USES_BYTES`]; the size read.
    TooLarge(usize),
    /// Not JSON, truncated, or a field missing or of the wrong type.
    Json(String),
    /// A schema number this reader does not know.
    Schema(u32),
    /// Two entries with one id.
    DuplicateId(String),
}

impl std::fmt::Display for UsesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge(size) => write!(
                f,
                "the uses table is {size} bytes; the most read is {MAX_USES_BYTES}"
            ),
            Self::Json(error) => write!(f, "the uses table is not valid: {error}"),
            Self::Schema(schema) => write!(
                f,
                "the uses table has schema {schema}; this tool reads schema {USES_SCHEMA}"
            ),
            Self::DuplicateId(id) => write!(f, "the uses table lists {id} twice"),
        }
    }
}

impl std::error::Error for UsesError {}

impl UsesTable {
    /// Reads a table: refuses an oversized document, an unknown schema, malformed JSON and
    /// repeated ids. Unknown fields are ignored. Entries come back sorted by id.
    pub fn read(bytes: &[u8]) -> Result<Self, UsesError> {
        #[derive(Deserialize)]
        struct Header {
            schema: u32,
        }
        if bytes.len() > MAX_USES_BYTES {
            return Err(UsesError::TooLarge(bytes.len()));
        }
        let header: Header =
            serde_json::from_slice(bytes).map_err(|e| UsesError::Json(e.to_string()))?;
        if header.schema != USES_SCHEMA {
            return Err(UsesError::Schema(header.schema));
        }
        let mut table: Self =
            serde_json::from_slice(bytes).map_err(|e| UsesError::Json(e.to_string()))?;
        table.entries.sort();
        if let Some(pair) = table
            .entries
            .windows(2)
            .find(|pair| pair[0].id == pair[1].id)
        {
            return Err(UsesError::DuplicateId(pair[0].id.clone()));
        }
        Ok(table)
    }

    /// The table as written: pretty JSON, entries sorted by id, ending in a newline.
    pub fn to_json(&self) -> String {
        let mut sorted = self.clone();
        sorted.entries.sort();
        let mut text = serde_json::to_string_pretty(&sorted).expect("a uses table serializes");
        text.push('\n');
        text
    }
}
