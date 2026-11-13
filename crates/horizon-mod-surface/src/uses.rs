//! What a mod uses of a surface, and whether it fits another build's surface (design §10.3).
//!
//! [`uses`] scans a mod's Rust source ([`crate::scan`]) and lists the surface records it names,
//! then every record their signatures reference, transitively ([`Surface::closure`]). A record
//! counts when its name occurs as an identifier in the source or the manifest names it, with two
//! narrowings:
//! - a member (a trait method, an inherent item, an associated item, a field, a variant, a hook's
//!   argument) counts only when its owner trait, type or hook is reached from what counts,
//!   through the supertraits of called traits too ([`Surface::reach`]): a sub-host trait's method
//!   called on `&mut dyn ServerHost` counts once `ServerHost` does;
//! - a name written after `X::`, where `X` owns a member of that name, is that member only
//!   (`TextHover::Item` is the variant, not a type `Item`).
//!
//! For each struct the source names it also records how the mod builds it: [`Built::Full`] when a
//! literal or pattern lists its fields with no `..` (`Name { … }`, `Name::<T> { … }`, `Name(…)`,
//! `Self { … }` in an impl for it, a local alias or renamed import of it, or a tuple or unit
//! struct's name used as a value), [`Built::Base`] when every one ends in `..` (and for a union,
//! whose literal names one field), [`Built::None`] otherwise. A name counts as built anywhere
//! but where only a type can stand. Which struct a name means is decided by two rules, with no
//! module resolution:
//! - a name written with a path (`a::b::Name`) is every API struct of that name;
//! - an unqualified name (or `Self` in an impl for one) is the mod's own type only in a file
//!   that defines it at its top level, binds it with no `use`, has no glob, no inline module, no
//!   `use` with a metavariable, and only standard or API macros and attributes whose names
//!   nothing in the mod rebinds. Otherwise it is every API struct of that name.
//!
//! A struct literal of a name the scan cannot place (a macro's `$t { … }`) builds every API
//! struct the mod names in full: when unsure, the scan says full, because a refusal is safe and
//! a wrong "fits" is not. It records the public paths the mod writes each item by, too,
//! through renamed or imported API modules as well.
//!
//! [`fits`] compares such a list with a target build's surface. The mod fits when every entry
//! exists there with an equal hash and under every path the mod writes, every type keeps the
//! trait impls it had, every struct the mod builds can still be built and keeps its field set
//! when built in full, and every hook whose version the manifest names keeps that version. The
//! per-member hashes make this exact about additions: a new `Host` method or sub-host trait, a
//! new hook, a new hook argument, a new field on a struct the mod does not build in full, or a
//! new trait impl changes nothing for the mod.
//!
//! Whether a mod is exact by nature (a `[native.*]` table, internals) is the caller's to decide
//! from its manifest before asking, and such a mod gets no table: this crate does not read
//! manifests.
//!
//! A [`UsesTable`] is the list a package carries: schema, the build it was made for, and its
//! entries. [`UsesTable::read`] refuses an unknown schema, malformed JSON, repeated ids and
//! documents over [`MAX_USES_BYTES`] with a typed [`UsesError`].
use crate::scan::{self, Token};
use crate::{MacroKind, Role, Surface, Type, TypeKind, leaf};
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
#[serde(deny_unknown_fields)]
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
    /// With [`Built::Full`]: its public fields' names at the made-for build, to name what the
    /// target gained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// With [`Built::Full`] or [`Built::Base`]: `file:line` of the first such literal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    /// The public paths the mod writes it by, other than its id's: the target must still
    /// export it under each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// For a trait's member: every called trait the mod counts whose supertraits lead to the
    /// member's owner (`trait:ServerHost` for a sub-host trait's method the mod may call on
    /// `&mut dyn ServerHost`), sorted. The target must still lead from each to the owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reached_from: Vec<String>,
    /// For a hook whose version the manifest names: that version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

/// An entry and why it is there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Used {
    pub entry: Entry,
    pub origin: Origin,
    /// What `check --uses` tells the author about how the mod builds this struct, when it
    /// builds it with a literal or pattern.
    pub note: Option<String>,
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

/// What the sources say about how a struct is built.
#[derive(Default)]
struct Building {
    full: Option<Place>,
    base: Option<Place>,
    /// Some full build is a brace expression (`S { a: 1 }`), not a pattern or a call.
    brace_literal: bool,
}

impl Building {
    fn record(&mut self, place: Place, full: bool, brace_literal: bool) {
        let slot = if full { &mut self.full } else { &mut self.base };
        if slot.as_ref().is_none_or(|old| place < *old) {
            *slot = Some(place);
        }
        self.brace_literal |= full && brace_literal;
    }
}

type Tokens<'t> = [(Token<'t>, usize)];

fn punct(tokens: &Tokens<'_>, at: usize, byte: u8) -> bool {
    tokens
        .get(at)
        .is_some_and(|(t, _)| *t == Token::Punct(byte))
}

fn ident<'t>(tokens: &Tokens<'t>, at: usize) -> Option<&'t str> {
    match tokens.get(at) {
        Some((Token::Ident(name), _)) => Some(name),
        _ => None,
    }
}

/// The first token of the path that ends at `last`: `a::b::c` from `c`'s index.
fn path_start(tokens: &Tokens<'_>, last: usize) -> usize {
    let mut first = last;
    while first >= 3
        && punct(tokens, first - 1, b':')
        && punct(tokens, first - 2, b':')
        && ident(tokens, first - 3).is_some()
    {
        first -= 3;
    }
    first
}

/// The index of the token that closes the bracket opened at `open`, if it closes.
fn closing(tokens: &Tokens<'_>, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, (token, _)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Punct(b'(' | b'[' | b'{') => depth += 1,
            Token::Punct(b')' | b']' | b'}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The index after the `<…>` that opens at `open`, angle brackets counted; `None` if it does not
/// close before a `;`, `{` or the end.
fn after_angles(tokens: &Tokens<'_>, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, (token, _)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Punct(b'<') => depth += 1,
            Token::Punct(b'>') if index > 0 && !punct(tokens, index - 1, b'-') => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            Token::Punct(b';' | b'{') => return None,
            _ => {}
        }
    }
    None
}

/// Whether the literal or pattern whose bracket opens at `open` and closes at `close` ends its
/// field list with `..` (a base expression or a rest pattern).
fn has_rest(tokens: &Tokens<'_>, open: usize, close: usize) -> bool {
    let mut depth = 0usize;
    let mut previous = tokens[open].0;
    for index in open + 1..close {
        let token = tokens[index].0;
        match token {
            Token::Punct(b'(' | b'[' | b'{') => depth += 1,
            Token::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            Token::Punct(b'.')
                if depth == 0
                    && punct(tokens, index + 1, b'.')
                    && matches!(previous, Token::Punct(b'{' | b'(' | b',')) =>
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

/// Whether the brace at `open` holds what a struct literal or pattern holds: nothing, `..`, or a
/// field name followed by `:`, `,` or `}`.
fn looks_like_fields(tokens: &Tokens<'_>, open: usize) -> bool {
    match tokens.get(open + 1).map(|(t, _)| *t) {
        Some(Token::Punct(b'}')) => true,
        Some(Token::Punct(b'.')) => punct(tokens, open + 2, b'.'),
        Some(Token::Ident(_)) => {
            punct(tokens, open + 2, b',')
                || punct(tokens, open + 2, b'}')
                || (punct(tokens, open + 2, b':') && !punct(tokens, open + 3, b':'))
        }
        _ => false,
    }
}

/// Whether the name at `at` stands where a type does, not a value: after `->`, `impl`, `for`,
/// `dyn`, `<`, a definition keyword or a keyword whose block follows it, path segments before it
/// skipped.
fn in_type_position(tokens: &Tokens<'_>, mut at: usize) -> bool {
    while at >= 3
        && punct(tokens, at - 1, b':')
        && punct(tokens, at - 2, b':')
        && ident(tokens, at - 3).is_some()
    {
        at -= 3;
    }
    match at.checked_sub(1).map(|i| tokens[i].0) {
        Some(Token::Ident(
            "impl" | "for" | "dyn" | "struct" | "enum" | "union" | "trait" | "type" | "as" | "mod"
            | "fn" | "where" | "if" | "match" | "while" | "in",
        )) => true,
        Some(Token::Punct(b'>')) => at >= 2 && punct(tokens, at - 2, b'-'),
        Some(Token::Punct(b'<')) => true,
        _ => false,
    }
}

/// One `use` tree from `at`: each path it imports with the name it is imported as. Returns the
/// index after the tree.
fn use_tree<'t>(
    tokens: &Tokens<'t>,
    mut at: usize,
    end: usize,
    prefix: &mut Vec<&'t str>,
    out: &mut Vec<(Vec<&'t str>, &'t str)>,
) -> usize {
    let depth = prefix.len();
    while at < end {
        match tokens[at].0 {
            Token::Ident(name) if name != "as" => {
                prefix.push(name);
                at += 1;
                if punct(tokens, at, b':') && punct(tokens, at + 1, b':') {
                    at += 2;
                    continue;
                }
                let mut alias = name;
                if ident(tokens, at) == Some("as") {
                    if let Some(local) = ident(tokens, at + 1) {
                        alias = local;
                    }
                    at += 2;
                }
                out.push((prefix.clone(), alias));
                break;
            }
            Token::Punct(b'{') => {
                at += 1;
                while at < end && !punct(tokens, at, b'}') {
                    at = use_tree(tokens, at, end, prefix, out);
                    if punct(tokens, at, b',') {
                        at += 1;
                    }
                }
                at += 1;
                break;
            }
            Token::Punct(b'*') => {
                let mut glob = prefix.clone();
                glob.push("*");
                out.push((glob, "*"));
                at += 1;
                break;
            }
            _ => {
                at += 1;
                break;
            }
        }
    }
    prefix.truncate(depth);
    at
}

/// What one source file holds that names resolve through.
struct FileFacts<'t> {
    tokens: Vec<(Token<'t>, usize)>,
    /// Tokens inside a `use` statement.
    in_use: Vec<bool>,
    /// Every `use` path with the name it brings in (`*` for a glob).
    imports: Vec<(Vec<&'t str>, &'t str)>,
    /// Every path written out in code, `a::b::C`, by the index of its last name.
    chains: Vec<Vec<&'t str>>,
    /// `type X = …Y<…>;` items: (X, Y), and whether it is an item of its own (not an associated
    /// type of an impl or trait).
    type_aliases: Vec<(&'t str, &'t str, bool)>,
    /// Each `impl` or `trait` body's token range, the type or trait it is for, and the path it
    /// is written by (`impl Make for mod_api::Settings` gives `mod_api`, `Settings`).
    bodies: Vec<Body<'t>>,
    /// The structs, enums, unions and type aliases the file defines at its top level.
    defines: BTreeSet<&'t str>,
    /// The file holds an inline `mod name { … }`, so it is more than one scope.
    inline_modules: bool,
    /// Some `use` holds a macro metavariable (`use mod_api::$m;`): what it binds is unknown.
    use_metavariable: bool,
    /// Every macro call anywhere in the file, function bodies and macro bodies included: the
    /// macro's path, or `["$"]` for one named by a metavariable.
    macro_calls: Vec<Vec<&'t str>>,
    /// Every attribute anywhere in the file, `#[…]` or `#![…]`: the token range inside its
    /// brackets.
    attributes: Vec<(usize, usize)>,
    /// The name of each `macro_rules!` definition.
    macro_rules: Vec<&'t str>,
}

/// An `impl` or `trait` body: its token range, its target's last name and the whole path.
type Body<'t> = (usize, usize, &'t str, Vec<&'t str>);

/// The deepest nesting of `use` groups the scan reads: `use a::{b::{c}}` nests 2.
pub const MAX_USE_NESTING: usize = 64;

/// Why a mod's source cannot be judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanError {
    /// A `use` statement nests its groups deeper than [`MAX_USE_NESTING`], at this line of
    /// this file.
    UseTooDeep { file: String, line: usize },
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UseTooDeep { file, line } => write!(
                f,
                "{file}:{line}: a `use` statement nests its groups deeper than {MAX_USE_NESTING}; the mod's source cannot be judged"
            ),
        }
    }
}

impl std::error::Error for ScanError {}

/// A file's facts, or the line of a `use` statement nested deeper than [`MAX_USE_NESTING`].
fn file_facts(text: &str) -> Result<FileFacts<'_>, usize> {
    let tokens: Vec<(Token<'_>, usize)> = scan::tokens(text).collect();
    let mut in_use = vec![false; tokens.len()];
    let mut imports = Vec::new();
    let mut chains = Vec::new();
    let mut type_aliases = Vec::new();
    let mut bodies: Vec<Body<'_>> = Vec::new();
    let mut use_metavariable = false;
    let mut at = 0;
    while at < tokens.len() {
        match tokens[at].0 {
            Token::Ident("use") => {
                let end = (at..tokens.len())
                    .find(|i| punct(&tokens, *i, b';'))
                    .unwrap_or(tokens.len());
                in_use[at..end].iter_mut().for_each(|t| *t = true);
                use_metavariable |= tokens[at..end]
                    .iter()
                    .any(|(token, _)| *token == Token::Punct(b'$'));
                // [`use_tree`] recurses once per group: bound it before it reads one.
                let mut nesting = 0usize;
                for (token, _) in &tokens[at..end] {
                    match token {
                        Token::Punct(b'{') => nesting += 1,
                        Token::Punct(b'}') => nesting = nesting.saturating_sub(1),
                        _ => {}
                    }
                    if nesting > MAX_USE_NESTING {
                        return Err(tokens[at].1);
                    }
                }
                let mut start = at + 1;
                while start < end {
                    if punct(&tokens, start, b':') {
                        start += 2;
                    }
                    start = use_tree(&tokens, start, end, &mut Vec::new(), &mut imports);
                }
                // `a::b::{self}` binds `b`, and `{self as x}` binds `x`, both to `a::b`.
                for (path, local) in imports.iter_mut() {
                    if path.len() >= 2 && path.last() == Some(&"self") {
                        path.pop();
                        if *local == "self" {
                            *local = path[path.len() - 1];
                        }
                    }
                }
            }
            Token::Ident("type") => {
                // `type X = path::Y<…>;`: X names Y.
                if let (Some(alias), Some(end)) = (
                    ident(&tokens, at + 1),
                    (at..tokens.len()).find(|i| punct(&tokens, *i, b';')),
                ) && let Some(eq) = (at..end).find(|i| punct(&tokens, *i, b'='))
                {
                    let target = (eq + 1..end)
                        .take_while(|i| !punct(&tokens, *i, b'<'))
                        .filter_map(|i| ident(&tokens, i))
                        .last();
                    let associated = bodies
                        .iter()
                        .any(|(open, close, _, _)| *open < at && at < *close);
                    if let Some(target) = target {
                        type_aliases.push((alias, target, !associated));
                    }
                }
            }
            // An `impl` or `trait` item, not `-> impl Trait` or `x: impl Trait`.
            Token::Ident(keyword @ ("impl" | "trait"))
                if at == 0
                    || matches!(
                        tokens[at - 1].0,
                        Token::Punct(b'}' | b';' | b'{' | b']' | b')')
                            | Token::Ident("unsafe" | "pub")
                    ) =>
            {
                // The type is the last name at angle depth 0 before the body or `where`, with
                // the path it ends.
                let mut i = at + 1;
                let mut target = None;
                let mut path = Vec::new();
                while i < tokens.len() {
                    match tokens[i].0 {
                        Token::Punct(b'<') => match after_angles(&tokens, i) {
                            Some(next) => {
                                i = next;
                                continue;
                            }
                            None => break,
                        },
                        Token::Punct(b'{') | Token::Ident("where") => break,
                        Token::Punct(b';') => {
                            target = None;
                            break;
                        }
                        Token::Punct(b':') if keyword == "trait" => break,
                        Token::Ident(name) if name != "for" && name != "dyn" => {
                            if !(i >= 2
                                && punct(&tokens, i - 1, b':')
                                && punct(&tokens, i - 2, b':'))
                            {
                                path.clear();
                            }
                            path.push(name);
                            target = Some(name);
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let open = (i..tokens.len()).find(|j| punct(&tokens, *j, b'{'));
                if let (Some(target), Some(open)) = (target, open)
                    && let Some(close) = closing(&tokens, open)
                {
                    bodies.push((open, close, target, path));
                }
            }
            Token::Ident(first)
                if !in_use[at]
                    && punct(&tokens, at + 1, b':')
                    && punct(&tokens, at + 2, b':')
                    && !(at >= 2
                        && punct(&tokens, at - 1, b':')
                        && punct(&tokens, at - 2, b':')) =>
            {
                // A path written out: `a::b::C`.
                let mut path = vec![first];
                let mut i = at + 1;
                while punct(&tokens, i, b':') && punct(&tokens, i + 1, b':') {
                    let Some(name) = ident(&tokens, i + 2) else {
                        break;
                    };
                    path.push(name);
                    i += 3;
                }
                chains.push(path);
            }
            _ => {}
        }
        at += 1;
    }
    let mut defines = BTreeSet::new();
    let mut inline_modules = false;
    let mut depth = 0usize;
    let mut macro_calls = Vec::new();
    let mut attributes = Vec::new();
    let mut macro_rules = Vec::new();
    for at in 0..tokens.len() {
        match tokens[at].0 {
            Token::Punct(b'!')
                if at > 0
                    && ident(&tokens, at - 1).is_some()
                    && matches!(
                        tokens.get(at + 1).map(|(t, _)| *t),
                        Some(Token::Punct(b'(' | b'[' | b'{') | Token::Ident(_))
                    ) =>
            {
                let first = path_start(&tokens, at - 1);
                if first >= 1 && punct(&tokens, first - 1, b'$') {
                    macro_calls.push(vec!["$"]);
                    continue;
                }
                let path: Vec<&str> = (first..at)
                    .step_by(3)
                    .filter_map(|i| ident(&tokens, i))
                    .collect();
                if path == ["macro_rules"]
                    && let Some(name) = ident(&tokens, at + 1)
                {
                    macro_rules.push(name);
                }
                macro_calls.push(path);
            }
            Token::Punct(b'#') => {
                let open = if punct(&tokens, at + 1, b'!') {
                    at + 2
                } else {
                    at + 1
                };
                if punct(&tokens, open, b'[') {
                    attributes.push((open, closing(&tokens, open).unwrap_or(tokens.len())));
                }
            }
            Token::Punct(b'{') => depth += 1,
            Token::Punct(b'}') => depth = depth.saturating_sub(1),
            Token::Ident("mod")
                if ident(&tokens, at + 1).is_some() && punct(&tokens, at + 2, b'{') =>
            {
                inline_modules = true;
            }
            Token::Ident("struct" | "enum" | "union" | "type") if depth == 0 => {
                if let Some(name) = ident(&tokens, at + 1) {
                    defines.insert(name);
                }
            }
            _ => {}
        }
    }
    Ok(FileFacts {
        tokens,
        in_use,
        imports,
        chains,
        type_aliases,
        bodies,
        defines,
        inline_modules,
        use_metavariable,
        macro_calls,
        attributes,
        macro_rules,
    })
}

/// What the whole mod says that each name resolves through.
struct ModFacts<'t> {
    /// The names the API crate is known by: `mod_api`, and any `use mod_api as x` or
    /// `extern crate mod_api as x`.
    roots: BTreeSet<&'t str>,
    /// Every local name some file gives another name, with each name it gives: any
    /// `use … as Y` (`Y` names the path's last segment) and any `type Y = …X<…>;`.
    aliases: BTreeMap<&'t str, BTreeSet<&'t str>>,
    /// API paths some file binds a name to, `mod_api::` left out, with each path: every `use`
    /// of an API path that is not a glob (`use mod_api::prelude as p;` binds `p` to
    /// `prelude`), and those of `use` paths that start at such a name, followed.
    bound: BTreeMap<&'t str, BTreeSet<Vec<&'t str>>>,
    /// The structs, enums, unions and enum variants the mod defines.
    types: BTreeSet<&'t str>,
    /// The type aliases the mod defines as items of their own.
    item_aliases: BTreeSet<&'t str>,
    /// Every other name the mod defines: traits, associated types.
    others: BTreeSet<&'t str>,
    /// Names some `macro_rules!` or some `use` that does not start at an API root binds: a
    /// standard or API macro or attribute of that name may be another.
    rebound: BTreeSet<&'t str>,
    /// Some glob outside the API and the mod (`use dep::*;`) or a `#[macro_use]` may bring in
    /// macros of any name.
    rebound_all: bool,
}

fn mod_facts<'t>(files: &[FileFacts<'t>]) -> ModFacts<'t> {
    let mut roots = BTreeSet::from(["mod_api"]);
    for file in files {
        for (path, name) in &file.imports {
            if path[..] == ["mod_api"] {
                roots.insert(*name);
            }
        }
        for at in 0..file.tokens.len() {
            if ident(&file.tokens, at) == Some("extern")
                && ident(&file.tokens, at + 2) == Some("mod_api")
                && ident(&file.tokens, at + 3) == Some("as")
                && let Some(name) = ident(&file.tokens, at + 4)
            {
                roots.insert(name);
            }
        }
    }
    let mut facts = ModFacts {
        roots,
        aliases: BTreeMap::new(),
        bound: BTreeMap::new(),
        types: BTreeSet::new(),
        item_aliases: BTreeSet::new(),
        others: BTreeSet::new(),
        rebound: BTreeSet::new(),
        rebound_all: false,
    };
    for file in files {
        facts.rebound.extend(file.macro_rules.iter().copied());
        facts.rebound_all |= file
            .attributes
            .iter()
            .any(|(open, _)| ident(&file.tokens, open + 1) == Some("macro_use"));
        for (path, name) in &file.imports {
            let first = path[0];
            let api = facts.roots.contains(first);
            if *name == "*" {
                facts.rebound_all |= !api && !matches!(first, "crate" | "self" | "super");
                continue;
            }
            if !api {
                facts.rebound.insert(name);
            }
            let last = path[path.len() - 1];
            if name != &last {
                facts.aliases.entry(name).or_default().insert(last);
            }
            if api && path.len() >= 2 {
                facts
                    .bound
                    .entry(name)
                    .or_default()
                    .insert(path[1..].to_vec());
            }
        }
        for (alias, target, item) in &file.type_aliases {
            facts.aliases.entry(alias).or_default().insert(target);
            if *item {
                facts.item_aliases.insert(alias);
            } else {
                facts.others.insert(alias);
            }
        }
        let tokens = &file.tokens;
        for at in 0..tokens.len() {
            let Some(keyword) = ident(tokens, at) else {
                continue;
            };
            if !matches!(keyword, "struct" | "enum" | "union" | "trait") {
                continue;
            }
            let Some(name) = ident(tokens, at + 1) else {
                continue;
            };
            if keyword == "trait" {
                facts.others.insert(name);
                continue;
            }
            facts.types.insert(name);
            if keyword != "enum" {
                continue;
            }
            let Some(open) = (at..tokens.len()).find(|i| punct(tokens, *i, b'{')) else {
                continue;
            };
            let close = closing(tokens, open).unwrap_or(tokens.len());
            let mut depth = 0usize;
            for i in open + 1..close {
                match tokens[i].0 {
                    Token::Punct(b'(' | b'[' | b'{') => depth += 1,
                    Token::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
                    Token::Ident(variant)
                        if depth == 0
                            && matches!(tokens[i - 1].0, Token::Punct(b'{' | b',' | b']')) =>
                    {
                        facts.types.insert(variant);
                    }
                    _ => {}
                }
            }
        }
    }
    // `use` paths that start at a name bound to an API path: `use p::Biome;` after
    // `use mod_api::prelude as p;`. Followed a bounded number of times.
    for _ in 0..4 {
        let mut more: Vec<(&str, Vec<&str>)> = Vec::new();
        for file in files {
            for (path, name) in &file.imports {
                if *name == "*" || path.len() < 2 {
                    continue;
                }
                for prefix in facts.bound.get(path[0]).into_iter().flatten() {
                    let mut full = prefix.clone();
                    full.extend(&path[1..]);
                    if !facts
                        .bound
                        .get(name)
                        .is_some_and(|known| known.contains(&full))
                    {
                        more.push((name, full));
                    }
                }
            }
        }
        if more.is_empty() {
            break;
        }
        for (name, path) in more {
            facts.bound.entry(name).or_default().insert(path);
        }
    }
    facts
}

/// Each API path (`mod_api::` left out) a path written in the mod may be: itself after an API
/// root, or after a name bound to an API path.
fn api_paths_of<'t>(path: &[&'t str], module: &ModFacts<'t>) -> Vec<Vec<&'t str>> {
    let Some((first, rest)) = path.split_first() else {
        return Vec::new();
    };
    if module.roots.contains(first) {
        return vec![rest.to_vec()];
    }
    module
        .bound
        .get(first)
        .into_iter()
        .flatten()
        .map(|prefix| {
            let mut full = prefix.clone();
            full.extend(rest);
            full
        })
        .collect()
}

/// The API names `name` may stand for: itself when it is one, and every name the mod's aliases
/// lead it to, followed transitively in any file.
fn api_names_of<'t, 'k>(
    name: &'t str,
    module: &ModFacts<'t>,
    by_name: &BTreeMap<&'k str, Vec<&'k str>>,
) -> BTreeSet<&'k str> {
    let mut out = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![name];
    while let Some(next) = pending.pop() {
        if !seen.insert(next) {
            continue;
        }
        if let Some((key, _)) = by_name.get_key_value(next) {
            out.insert(*key);
        }
        pending.extend(module.aliases.get(next).into_iter().flatten().copied());
    }
    out
}

/// Standard macros that cannot bring a name into scope: they expand to an expression, a string
/// or an error, never to a `use` or a `mod`.
const STD_MACROS: &[&str] = &[
    "include_str",
    "include_bytes",
    "compile_error",
    "concat",
    "env",
    "option_env",
    "stringify",
    "line",
    "column",
    "file",
    "module_path",
    "cfg",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "format",
    "format_args",
    "vec",
    "matches",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "dbg",
];

/// Attributes the compiler itself reads: none of them expands to items.
const BUILTIN_ATTRIBUTES: &[&str] = &[
    "derive",
    "cfg",
    "cfg_attr",
    "test",
    "allow",
    "warn",
    "deny",
    "forbid",
    "expect",
    "doc",
    "inline",
    "must_use",
    "repr",
    "non_exhaustive",
    "path",
    "deprecated",
    "cold",
    "no_mangle",
    "export_name",
    "link_section",
    "used",
    "track_caller",
    "ignore",
    "should_panic",
    "macro_export",
    "macro_use",
    "automatically_derived",
    "rustfmt",
    "clippy",
    "diagnostic",
    "unsafe",
    "target_feature",
    "link",
];

/// The derives the standard library defines.
const STD_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

/// The macro, attribute and derive names a file may use and still be read as written: the
/// standard ones and the API's own, less every name something in the mod may rebind.
struct Trusted<'s> {
    roots: &'s BTreeSet<&'s str>,
    /// By kind, the API's own macro names.
    api_macros: BTreeSet<&'s str>,
    api_attributes: BTreeSet<&'s str>,
    api_derives: BTreeSet<&'s str>,
    rebound: &'s BTreeSet<&'s str>,
    rebound_all: bool,
}

/// What a macro, attribute or derive path is.
#[derive(Clone, Copy)]
enum Named {
    Macro,
    Attribute,
    Derive,
}

impl Trusted<'_> {
    /// Whether `path`, used as a `kind`, is certainly a standard or API one.
    fn path(&self, path: &[&str], kind: Named) -> bool {
        let (std, api) = match kind {
            Named::Macro => (STD_MACROS, &self.api_macros),
            Named::Attribute => (BUILTIN_ATTRIBUTES, &self.api_attributes),
            Named::Derive => (STD_DERIVES, &self.api_derives),
        };
        match path {
            [name] => {
                !self.rebound_all
                    && !self.rebound.contains(name)
                    && (std.contains(name)
                        || api.contains(name)
                        || (matches!(kind, Named::Macro) && *name == "macro_rules"))
            }
            [root, .., name] if self.roots.contains(root) => api.contains(name),
            [root, .., name] if matches!(*root, "std" | "core" | "alloc") => std.contains(name),
            [root, ..] => {
                matches!(kind, Named::Attribute)
                    && matches!(*root, "rustfmt" | "clippy" | "diagnostic")
            }
            [] => false,
        }
    }

    /// Whether the attribute written in `tokens[start..end]` is certainly a standard or API one,
    /// reading `derive(…)` and `cfg_attr(…)` arguments to `depth` levels.
    fn attribute(&self, tokens: &Tokens<'_>, start: usize, end: usize, depth: usize) -> bool {
        let (path, next) = path_at(tokens, start, end);
        if path.is_empty() || depth == 0 {
            return false;
        }
        let arguments = || {
            punct(tokens, next, b'(')
                .then(|| closing(tokens, next))
                .flatten()
                .map(|close| arguments(tokens, next, close))
        };
        match &path[..] {
            ["derive"] if self.path(&path, Named::Attribute) => arguments().is_some_and(|list| {
                list.iter().all(|(from, to)| {
                    let (derive, after) = path_at(tokens, *from, *to);
                    after == *to && self.path(&derive, Named::Derive)
                })
            }),
            ["cfg_attr"] if self.path(&path, Named::Attribute) => arguments().is_some_and(|list| {
                list.iter()
                    .skip(1)
                    .all(|(from, to)| self.attribute(tokens, *from, *to, depth - 1))
            }),
            _ => self.path(&path, Named::Attribute),
        }
    }

    /// Whether every macro call and attribute in `file` is certainly a standard or API one, so
    /// nothing the scan cannot expand may bring a name into its scope.
    fn file(&self, file: &FileFacts<'_>) -> bool {
        file.macro_calls
            .iter()
            .all(|path| self.path(path, Named::Macro))
            && file
                .attributes
                .iter()
                .all(|(open, close)| self.attribute(&file.tokens, open + 1, *close, 16))
    }
}

/// The path written from `at` before `end` (a leading `::` allowed), and the index after it;
/// empty when it does not start with a name.
fn path_at<'t>(tokens: &Tokens<'t>, mut at: usize, end: usize) -> (Vec<&'t str>, usize) {
    if punct(tokens, at, b':') && punct(tokens, at + 1, b':') {
        at += 2;
    }
    let mut path = Vec::new();
    while at < end
        && let Some(name) = ident(tokens, at)
    {
        path.push(name);
        at += 1;
        if at < end && punct(tokens, at, b':') && punct(tokens, at + 1, b':') {
            at += 2;
        } else {
            break;
        }
    }
    (path, at)
}

/// The comma-separated arguments inside the bracket `open`..`close`, each as a token range.
fn arguments(tokens: &Tokens<'_>, open: usize, close: usize) -> Vec<(usize, usize)> {
    let mut list = Vec::new();
    let mut depth = 0usize;
    let mut from = open + 1;
    for at in open + 1..close {
        match tokens[at].0 {
            Token::Punct(b'(' | b'[' | b'{') => depth += 1,
            Token::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            Token::Punct(b',') if depth == 0 => {
                list.push((from, at));
                from = at + 1;
            }
            _ => {}
        }
    }
    if from < close {
        list.push((from, close));
    }
    list
}

/// The bracket that encloses the token at `at`, as its index, if any.
fn enclosing(tokens: &Tokens<'_>, at: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..at).rev() {
        match tokens[index].0 {
            Token::Punct(b')' | b']' | b'}') => depth += 1,
            Token::Punct(b'(' | b'[' | b'{') if depth > 0 => depth -= 1,
            Token::Punct(b'(' | b'[' | b'{') => return Some(index),
            _ => {}
        }
    }
    None
}

/// How far back the position checks below look, in tokens: past it they answer "a value",
/// which counts a build, the safe side.
const LOOK_BACK: usize = 4096;

/// The bracket that encloses the token at `at`, angle brackets of generics counted (a `<`
/// after `->` or `=>` is no bracket); `None` past a `;` or [`LOOK_BACK`].
fn opener(tokens: &Tokens<'_>, at: usize) -> Option<usize> {
    let (mut depth, mut angles) = (0usize, 0usize);
    for index in (at.saturating_sub(LOOK_BACK)..at).rev() {
        match tokens[index].0 {
            Token::Punct(b')' | b']' | b'}') => depth += 1,
            Token::Punct(b'(' | b'[' | b'{') if depth > 0 => depth -= 1,
            Token::Punct(b'(' | b'[' | b'{') => return Some(index),
            Token::Punct(b'>')
                if depth == 0
                    && !(index > 0
                        && (punct(tokens, index - 1, b'-') || punct(tokens, index - 1, b'='))) =>
            {
                angles += 1;
            }
            Token::Punct(b'<') if depth == 0 => match angles.checked_sub(1) {
                Some(left) => angles = left,
                None => return Some(index),
            },
            Token::Punct(b';') if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// The keyword of the item whose name and generics come before `open` (`struct`, `union` or
/// `enum`), if one does before a `;`, `{`, `}`, `=` or `,`; with the token it stopped at.
fn item_keyword<'t>(tokens: &Tokens<'t>, open: usize) -> (Option<&'t str>, Option<usize>) {
    for index in (open.saturating_sub(256)..open).rev() {
        match tokens[index].0 {
            Token::Ident(keyword @ ("struct" | "union" | "enum")) => return (Some(keyword), None),
            Token::Punct(b';' | b'{' | b'}' | b'=' | b',') => return (None, Some(index)),
            _ => {}
        }
    }
    (None, None)
}

/// Whether the bracket at `open` holds a definition's fields: a struct's or union's body or
/// tuple, or an enum variant's.
fn definition_body(tokens: &Tokens<'_>, open: usize) -> bool {
    match item_keyword(tokens, open) {
        (Some(_), _) => true,
        (None, Some(stop)) if matches!(tokens[stop].0, Token::Punct(b'{' | b',')) => {
            enclosing(tokens, open).is_some_and(|outer| {
                punct(tokens, outer, b'{') && item_keyword(tokens, outer).0 == Some("enum")
            })
        }
        _ => false,
    }
}

/// Whether the `<` at `open` opens generics: after a capitalised name, a `::` (turbofish) or a
/// definition's name.
fn generic(tokens: &Tokens<'_>, open: usize) -> bool {
    match open.checked_sub(1).map(|i| tokens[i].0) {
        Some(Token::Ident(name)) => {
            name.starts_with(|c: char| c.is_ascii_uppercase())
                || matches!(name, "impl" | "fn")
                || (open >= 2
                    && matches!(
                        ident(tokens, open - 2),
                        Some("fn" | "struct" | "enum" | "union" | "trait" | "type" | "impl")
                    ))
        }
        Some(Token::Punct(b':')) => open >= 2 && punct(tokens, open - 2, b':'),
        _ => false,
    }
}

/// Whether the statement holding the `=` at `eq` is a `type` alias, whose right side is a type.
fn type_alias_eq(tokens: &Tokens<'_>, eq: usize) -> bool {
    for index in (eq.saturating_sub(256)..eq).rev() {
        match tokens[index].0 {
            Token::Ident("type") => return true,
            Token::Punct(b';' | b'{' | b'}') | Token::Ident("let") => return false,
            _ => {}
        }
    }
    false
}

/// Whether the path that starts at `start` stands only where a type does, never a value: after
/// `->`, a definition keyword, `impl`, `dyn`, `as` or `for`, a `:` that is not a struct
/// literal's field, the `=` of a `type` alias, inside generics, a function type's arguments, a
/// definition's fields, or a tuple, array or reference type in such a place. Anything else is a
/// value, which counts a build.
fn type_only(tokens: &Tokens<'_>, start: usize) -> bool {
    let mut at = start;
    for _ in 0..LOOK_BACK {
        let Some(previous) = at.checked_sub(1) else {
            return false;
        };
        match tokens[previous].0 {
            Token::Ident(
                "impl" | "dyn" | "struct" | "enum" | "union" | "trait" | "type" | "as" | "for"
                | "where" | "mod" | "fn" | "use",
            ) => return true,
            Token::Punct(b'>') => return previous >= 1 && punct(tokens, previous - 1, b'-'),
            Token::Punct(b'=') => return type_alias_eq(tokens, previous),
            Token::Punct(b':') => {
                // A struct literal's field: `{ name: N` or `, name: N` in a brace that holds no
                // definition's fields.
                let field = previous >= 2
                    && ident(tokens, previous - 1).is_some()
                    && matches!(tokens[previous - 2].0, Token::Punct(b'{' | b','))
                    && enclosing(tokens, previous).is_some_and(|open| {
                        punct(tokens, open, b'{') && !definition_body(tokens, open)
                    });
                return !field;
            }
            Token::Punct(b'<') => return generic(tokens, previous),
            Token::Punct(b'&' | b'*') | Token::Ident("mut" | "const") | Token::Lifetime => {
                at = previous;
            }
            Token::Punct(b',' | b'(' | b'[' | b'{') => {
                let open = if punct(tokens, previous, b',') {
                    match opener(tokens, previous) {
                        Some(open) => open,
                        None => return false,
                    }
                } else {
                    previous
                };
                match tokens[open].0 {
                    Token::Punct(b'<') => return generic(tokens, open),
                    Token::Punct(b'(')
                        if open >= 1
                            && matches!(
                                ident(tokens, open - 1),
                                Some("Fn" | "FnMut" | "FnOnce" | "fn")
                            ) =>
                    {
                        return true;
                    }
                    Token::Punct(b'(' | b'{') if definition_body(tokens, open) => return true,
                    Token::Punct(b'(' | b'[') => at = open,
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    false
}

/// The records `sources` (each a path and its Rust text) and `manifest` bring in, with
/// everything their signatures reference; [`ScanError`] when the source cannot be judged.
pub fn uses<P: AsRef<str>, T: AsRef<str>>(
    sources: impl IntoIterator<Item = (P, T)>,
    manifest: &ManifestFacts,
    surface: &Surface,
) -> Result<Uses, ScanError> {
    let hashes: BTreeMap<&str, &str> = surface.entries().collect();
    let types: BTreeMap<&str, &Type> = surface.types.iter().map(|t| (t.id.as_str(), t)).collect();
    let roles: BTreeMap<&str, Role> = surface
        .traits
        .iter()
        .map(|t| (t.id.as_str(), t.contract.role))
        .collect();
    let mut public_paths: BTreeMap<&str, &[String]> = BTreeMap::new();
    for t in &surface.traits {
        public_paths.insert(&t.id, &t.paths);
    }
    for t in &surface.types {
        public_paths.insert(&t.id, &t.paths);
    }
    for f in &surface.functions {
        public_paths.insert(&f.id, &f.paths);
    }
    for m in &surface.macros {
        public_paths.insert(&m.id, &m.paths);
    }
    let mut by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, id) in surface.identifiers() {
        by_name.entry(name).or_default().push(id);
    }
    let owners: BTreeMap<&str, String> = hashes
        .keys()
        .filter_map(|id| owner(id, &hashes).map(|owner| (*id, owner)))
        .collect();
    let kind = |id: &str| types.get(id).map(|t| t.contract.kind);
    let struct_like = |id: &str| {
        matches!(
            kind(id),
            Some(TypeKind::Struct | TypeKind::TupleStruct | TypeKind::UnitStruct | TypeKind::Union)
        )
    };

    let sources: Vec<(String, String)> = sources
        .into_iter()
        .map(|(p, t)| (p.as_ref().to_owned(), t.as_ref().to_owned()))
        .collect();
    let files: Vec<FileFacts<'_>> = sources
        .iter()
        .map(|(file, text)| {
            file_facts(text).map_err(|line| ScanError::UseTooDeep {
                file: file.clone(),
                line,
            })
        })
        .collect::<Result<_, _>>()?;
    let module = mod_facts(&files);
    let trusted = Trusted {
        roots: &module.roots,
        api_macros: surface
            .macros
            .iter()
            .filter(|m| {
                matches!(
                    m.contract.kind,
                    MacroKind::Function | MacroKind::Declarative
                )
            })
            .map(|m| leaf(&m.id))
            .collect(),
        api_attributes: surface
            .macros
            .iter()
            .filter(|m| matches!(m.contract.kind, MacroKind::Attribute | MacroKind::Derive))
            .flat_map(|m| {
                let own = (m.contract.kind == MacroKind::Attribute).then(|| leaf(&m.id));
                own.into_iter()
                    .chain(m.contract.attributes.iter().map(String::as_str))
            })
            .collect(),
        api_derives: surface
            .macros
            .iter()
            .filter(|m| m.contract.kind == MacroKind::Derive)
            .map(|m| leaf(&m.id))
            .collect(),
        rebound: &module.rebound,
        rebound_all: module.rebound_all,
    };
    // Whether nothing the scan cannot follow may change what names mean in each file: no glob,
    // no inline module, no `use` with a metavariable, and only standard or API macros and
    // attributes.
    let certain: Vec<bool> = files
        .iter()
        .map(|file| {
            !file.inline_modules
                && !file.use_metavariable
                && !file.imports.iter().any(|(_, local)| *local == "*")
                && trusted.file(file)
        })
        .collect();
    // Whether an unqualified `name` in a file certainly means the mod's own type: the file
    // defines it at its top level (a `type` alias of an API item is no definition), no `use` in
    // it binds the name, and the file is certain. Anything else leaves the API's.
    let own_in = |file: &FileFacts<'_>, certain: bool, name: &str| {
        certain
            && file.defines.contains(name)
            && !file.type_aliases.iter().any(|(alias, target, item)| {
                *item && *alias == name && !api_names_of(target, &module, &by_name).is_empty()
            })
            && !file.imports.iter().any(|(_, local)| *local == name)
    };

    // The records named directly, each with the first place that names it; how each named
    // struct is built; the public paths the mod writes each by.
    let mut direct: BTreeMap<&str, Origin> = BTreeMap::new();
    let mut building: BTreeMap<&str, Building> = BTreeMap::new();
    let mut written: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut named_structs: BTreeSet<&str> = BTreeSet::new();
    let mut unsure: Option<Place> = None;
    for (((path, _), file), certain) in sources.iter().zip(&files).zip(certain) {
        let tokens = &file.tokens;
        for (at, (token, line)) in tokens.iter().enumerate() {
            let place = || (path.clone(), *line);
            let Token::Ident(written_name) = token else {
                continue;
            };
            let start = path_start(tokens, at);
            // The name this token stands for (an impl's target for `Self`), whether it is the
            // mod's own type there, and every API name it may be. A name written with a path
            // is never the mod's own.
            let (name, own_here) = match *written_name {
                "Self" => match file
                    .bodies
                    .iter()
                    .filter(|(open, close, _, _)| *open < at && at < *close)
                    .min_by_key(|(open, close, _, _)| close - open)
                {
                    Some((_, _, target, path)) => (*target, path.len() == 1),
                    None => continue,
                },
                name => (name, start == at),
            };
            let api_names = api_names_of(name, &module, &by_name);
            let aliased = api_names.iter().any(|api| *api != name);
            let own_here = own_here && !aliased && own_in(file, certain, name);
            let ids: Vec<&str> = api_names
                .iter()
                .flat_map(|api| by_name[api].iter().copied())
                .collect();
            if ids.is_empty() {
                // A literal of a name the scan cannot place may be an API struct by another
                // name; so may a macro body's `$name { … }`.
                let unknown = name.starts_with(|c: char| c.is_ascii_uppercase())
                    && name.chars().any(|c| c.is_ascii_lowercase())
                    && !module.types.contains(name)
                    && !module.item_aliases.contains(name)
                    && !module.others.contains(name)
                    && *written_name != "Self";
                let metavariable = at >= 1 && punct(tokens, at - 1, b'$');
                let literal = punct(tokens, at + 1, b'{')
                    && !file.in_use[at]
                    && !in_type_position(tokens, at);
                if literal
                    && ((unknown && looks_like_fields(tokens, at + 1))
                        || (metavariable && !(at >= 2 && in_type_position(tokens, at - 1))))
                {
                    let here = place();
                    if unsure.as_ref().is_none_or(|old| here < *old) {
                        unsure = Some(here);
                    }
                }
                continue;
            }
            let qualifier = (start < at).then(|| ident(tokens, at - 3)).flatten();
            let members: Vec<&str> = qualifier
                .filter(|_| !aliased)
                .map(|q| {
                    ids.iter()
                        .copied()
                        .filter(|id| owners.get(id).is_some_and(|o| leaf(o) == q))
                        .collect()
                })
                .unwrap_or_default();
            let named = if members.is_empty() { ids } else { members };
            let here = Origin::Source {
                file: path.clone(),
                line: *line,
            };
            for id in &named {
                direct
                    .entry(id)
                    .and_modify(|origin| *origin = here.clone().min(origin.clone()))
                    .or_insert_with(|| here.clone());
                if !struct_like(id) || own_here {
                    continue;
                }
                named_structs.insert(id);
                if file.in_use[at] || type_only(tokens, start) {
                    continue;
                }
                let union = kind(id) == Some(TypeKind::Union);
                // A turbofish: `Name::<T> { … }`.
                let mut next = at + 1;
                if punct(tokens, next, b':')
                    && punct(tokens, next + 1, b':')
                    && punct(tokens, next + 2, b'<')
                {
                    match after_angles(tokens, next + 2) {
                        Some(after) => next = after,
                        None => {
                            building.entry(id).or_default().record(place(), true, false);
                            continue;
                        }
                    }
                }
                if punct(tokens, next, b'{') || punct(tokens, next, b'(') {
                    let brace = punct(tokens, next, b'{');
                    let Some(close) = closing(tokens, next) else {
                        // Unclosed: unsure, so full.
                        building.entry(id).or_default().record(place(), true, false);
                        continue;
                    };
                    let rest = has_rest(tokens, next, close);
                    // A pattern is followed by `=`, `=>`, `:` or `|`; an expression is not.
                    let pattern = matches!(
                        tokens.get(close + 1).map(|(t, _)| *t),
                        Some(Token::Punct(b'=' | b':' | b'|'))
                    ) && !punct(tokens, close + 2, b'=');
                    if pattern && rest {
                        // `let S { a, .. } = s`: a read of the named fields, not a build.
                        continue;
                    }
                    let full = !union && !rest;
                    building
                        .entry(id)
                        .or_default()
                        .record(place(), full, brace && !pattern);
                } else if next == at + 1
                    && matches!(kind(id), Some(TypeKind::TupleStruct | TypeKind::UnitStruct))
                    && !(punct(tokens, next, b':') && punct(tokens, next + 1, b':'))
                    && !punct(tokens, next, b'!')
                {
                    // A tuple struct's constructor or a unit struct used as a value.
                    building.entry(id).or_default().record(place(), true, false);
                }
            }
        }
        // The API paths this file writes: imported, written out, or under an imported glob,
        // each through an API root or a name bound to an API path.
        let mut paths: Vec<(String, &str)> = Vec::new();
        let mut globs: BTreeSet<String> = BTreeSet::new();
        for (segments, local) in &file.imports {
            if *local == "*" {
                for api in api_paths_of(&segments[..segments.len() - 1], &module) {
                    globs.insert(api.join("::"));
                }
                continue;
            }
            for api in api_paths_of(segments, &module) {
                if let Some(last) = api.last() {
                    paths.push((api.join("::"), last));
                }
            }
        }
        for chain in &file.chains {
            for api in api_paths_of(chain, &module) {
                if let Some(last) = api.last() {
                    paths.push((api.join("::"), last));
                }
            }
        }
        if !globs.is_empty() {
            let names: BTreeSet<&str> = tokens
                .iter()
                .filter_map(|(t, _)| match t {
                    Token::Ident(name) => Some(*name),
                    _ => None,
                })
                .collect();
            for glob in &globs {
                for name in &names {
                    let path = if glob.is_empty() {
                        (*name).to_owned()
                    } else {
                        format!("{glob}::{name}")
                    };
                    paths.push((path, name));
                }
            }
        }
        for (path, name) in paths {
            for id in by_name.get(name).into_iter().flatten() {
                let id_path = id.split_once(':').map_or("", |(_, rest)| rest);
                if path != id_path
                    && public_paths
                        .get(id)
                        .is_some_and(|paths| paths.contains(&path))
                {
                    written.entry(id).or_default().insert(path.clone());
                }
            }
        }
    }
    // A literal the scan cannot place, anywhere in the mod, may build any API struct the mod
    // names, anywhere: each counts as built field by field.
    if let Some(place) = unsure {
        for id in named_structs {
            building
                .entry(id)
                .or_default()
                .record(place.clone(), true, false);
        }
    }
    for name in &manifest.names {
        for id in [format!("hook:{name}"), format!("capability:{name}")] {
            if let Some((id, _)) = hashes.get_key_value(id.as_str()) {
                direct.entry(id).or_insert(Origin::Manifest);
            }
        }
    }

    // A member counts once its owner is reached from what counts, through the supertraits of
    // called traits too (a sub-host trait's method called on `&mut dyn ServerHost`).
    let mut counted: BTreeSet<&str> = direct
        .keys()
        .filter(|id| !owners.contains_key(*id))
        .copied()
        .collect();
    loop {
        let reached = surface.reach(counted.iter().copied());
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

    // The list: what counts and what its signatures reference, each reached record naming the
    // counted record it was first reached from. A called trait's supertraits are not in it.
    let refs = surface.refs(false);
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
    // A trait's member names every called trait in the list that leads to its owner through
    // supertraits, whether or not the mod also names the owner or the called trait: the scan
    // cannot see which one a call goes through (`host.player(id)` on `&mut dyn ServerHost` in a
    // mod that also counts `ClientHost`, `show(host)` upcasting to a named `&dyn PlayersHost`, or
    // a closure's host typed by an API method's signature), so the target must still lead from
    // each.
    let called: Vec<&str> = origins
        .keys()
        .copied()
        .filter(|id| roles.get(id) == Some(&Role::Call))
        .collect();
    let leads: BTreeMap<&str, BTreeSet<String>> = called
        .iter()
        .map(|start| (*start, surface.reach([*start])))
        .collect();
    let reached_from: BTreeMap<&str, Vec<&str>> = counted
        .iter()
        .filter_map(|id| {
            let owner = owners.get(id).filter(|o| o.starts_with("trait:"))?;
            let from: Vec<&str> = called
                .iter()
                .copied()
                .filter(|start| *start != owner.as_str() && leads[start].contains(owner))
                .collect();
            (!from.is_empty()).then_some((*id, from))
        })
        .collect();

    let at = |(file, line): &Place| format!("{file}:{line}");
    let hook_versions = &manifest.hook_versions;
    Ok(Uses {
        items: origins
            .into_iter()
            .map(|(id, origin)| {
                let record = types.get(id);
                let how = building.get(id);
                let built = struct_like(id).then_some(match how {
                    Some(Building { full: Some(_), .. }) => Built::Full,
                    Some(Building { base: Some(_), .. }) => Built::Base,
                    _ => Built::None,
                });
                let built_at = how
                    .and_then(|b| b.full.as_ref().or(b.base.as_ref()))
                    .map(at);
                let name = id.strip_prefix("type:").unwrap_or(id);
                let note = match (built, &built_at) {
                    (Some(Built::Base), Some(place)) => Some(format!(
                        "`{name}`: built with `..` ({place}), so a new field in a later game build will not break this mod."
                    )),
                    (Some(Built::Full), Some(place))
                        if how.is_some_and(|b| b.brace_literal)
                            && record.is_some_and(|t| t.impls.iter().any(|i| i == "Default")) =>
                    {
                        Some(format!(
                            "`{name}`: built field by field ({place}). A new field in a later game build will break this line; end the literal with `..Default::default()` to stay compatible."
                        ))
                    }
                    (Some(Built::Full), Some(place)) => Some(format!(
                        "`{name}`: built field by field ({place}). A new field in a later game build will break this line."
                    )),
                    _ => None,
                };
                Used {
                    entry: Entry {
                        id: id.to_owned(),
                        hash: hashes[id].to_owned(),
                        impls: record.map(|t| t.impls.clone()).unwrap_or_default(),
                        fields_hash: (built == Some(Built::Full))
                            .then(|| record.and_then(|t| t.fields_hash.clone()))
                            .flatten(),
                        fields: match (built, record) {
                            (Some(Built::Full), Some(t)) => {
                                t.fields.iter().map(|f| leaf(&f.id).to_owned()).collect()
                            }
                            _ => Vec::new(),
                        },
                        built,
                        built_at,
                        paths: written
                            .get(id)
                            .map(|paths| paths.iter().cloned().collect())
                            .unwrap_or_default(),
                        version: id
                            .strip_prefix("hook:")
                            .and_then(|name| hook_versions.get(name).copied()),
                        reached_from: reached_from
                            .get(id)
                            .into_iter()
                            .flatten()
                            .map(|from| (*from).to_owned())
                            .collect(),
                    },
                    origin,
                    note,
                }
            })
            .collect(),
    })
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
    /// The mod builds the struct with a literal or pattern at `at`, and the target's can no
    /// longer be (a private field, `#[non_exhaustive]`).
    NotBuildable { at: String },
    /// The target no longer exports it under this path (`mod_api::` left out).
    Path(String),
    /// The trait the mod reaches this member through no longer includes the member's owner.
    Route { from: String, owner: String },
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
            Change::NotBuildable { at } => write!(
                f,
                "{item} can no longer be built with a literal, and `{at}` builds it"
            ),
            Change::Route { from, owner } => write!(
                f,
                "`{}` no longer includes `{}`, which this mod calls `{}` on",
                leaf(from),
                leaf(owner),
                leaf(&self.id)
            ),
            Change::Path(path) => {
                write!(f, "`mod_api::{path}` is no longer exported under that path")
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

/// Whether every entry exists in `target` with an equal hash and under every path the mod writes,
/// every type keeps its impls, every struct the mod builds can still be built (and keeps its
/// field set when built in full), and every named hook version is unchanged.
pub fn fits<'a>(entries: impl IntoIterator<Item = &'a Entry>, target: &Surface) -> Fit {
    let hashes: BTreeMap<&str, &str> = target.entries().collect();
    let types: BTreeMap<&str, &Type> = target.types.iter().map(|t| (t.id.as_str(), t)).collect();
    let versions: BTreeMap<&str, u64> = target
        .hooks
        .iter()
        .map(|h| (h.id.as_str(), h.contract.version))
        .collect();
    let mut public_paths: BTreeMap<&str, &[String]> = BTreeMap::new();
    for t in &target.traits {
        public_paths.insert(&t.id, &t.paths);
    }
    for t in &target.types {
        public_paths.insert(&t.id, &t.paths);
    }
    for f in &target.functions {
        public_paths.insert(&f.id, &f.paths);
    }
    for m in &target.macros {
        public_paths.insert(&m.id, &m.paths);
    }
    let entries: Vec<&Entry> = entries.into_iter().collect();
    // What each trait a member is reached from leads to in the target.
    let mut leads: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
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
        for path in &entry.paths {
            if !public_paths
                .get(id)
                .is_some_and(|paths| paths.contains(path))
            {
                misfit(Change::Path(path.clone()));
            }
        }
        if let Some(record) = types.get(id) {
            for name in &entry.impls {
                if !record.impls.contains(name) {
                    misfit(Change::LostImpl(name.clone()));
                }
            }
            if matches!(entry.built, Some(Built::Full | Built::Base))
                && !record.contract.constructible
            {
                misfit(Change::NotBuildable {
                    at: entry.built_at.clone().unwrap_or_default(),
                });
            }
            if entry.built == Some(Built::Full) && entry.fields_hash != record.fields_hash {
                let gained = record
                    .fields
                    .iter()
                    .map(|f| leaf(&f.id).to_owned())
                    .filter(|name| !entry.fields.contains(name))
                    .collect();
                misfit(Change::Fields {
                    gained,
                    at: entry.built_at.clone().unwrap_or_default(),
                });
            }
        }
        if let Some(owner) = owner(id, &hashes) {
            for from in &entry.reached_from {
                let leads = leads
                    .entry(from.as_str())
                    .or_insert_with(|| target.reach([from.as_str()]));
                if !leads.contains(&owner) {
                    misfit(Change::Route {
                        from: from.clone(),
                        owner: owner.clone(),
                    });
                }
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
#[serde(deny_unknown_fields)]
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
    /// repeated ids, and unknown fields: the table is this crate's own format, and a field a newer
    /// writer adds is a check this reader would silently skip. Entries come back sorted by id.
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
