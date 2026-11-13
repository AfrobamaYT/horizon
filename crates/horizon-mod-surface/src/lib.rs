//! `surface.json`: the stable surface of one game build, what an API-only mod can name.
//!
//! The generator (`horizon-stage surface`) writes it from the declarations the build parses;
//! nothing in it is kept by hand. This crate holds its record types, their contract hashes, the
//! strict reader and the one shape function. It has no engine or parser dependency and builds
//! for `wasm32-unknown-unknown`.
//!
//! # Ids
//!
//! Every record and every member record has one id, unique in the document. Paths are the path
//! a mod writes after `mod_api::` (the shortest public one; the first in byte order of equals).
//!
//! | Record | Id |
//! |---|---|
//! | hook | `hook:<name>` |
//! | trait | `trait:<path>` |
//! | trait method | `method:<trait path>::<name>` |
//! | inherent method of an API type | `handle:<type path>::<name>` |
//! | free function | `fn:<path>` |
//! | macro | `macro:<path>` |
//! | type | `type:<path>` |
//! | public field or variant of a type | `type:<path>.<field or variant>` (a tuple field by position) |
//! | Part kind | `part:<kind>` |
//! | capability | `capability:<key>` (`registries`, `horizon:fluids`) |
//!
//! # Hashes
//!
//! A record's `hash` is [`contract_hash`] of its contract: every field except `id`, `hash`,
//! `doc`, a hook's `engine` and `counterparts`, a type's `shape`, and its member records. Each
//! member (a method, field or variant) has its own hash, so adding a method to `Host`, adding a
//! hook or adding a field to a type mods cannot build changes no existing record's hash. A
//! container's hash covers its members only where a new member breaks users: the fields of a
//! `constructible` struct, the variants of an exhaustive enum, the required methods of a trait
//! mods implement. A type's `shape` is its full wire layout ([`shape`]) and is not hashed into
//! its contract. The document's `hash` is [`Surface::document_hash`] over every (id, hash).
//!
//! # Reading
//!
//! [`Surface::read`] refuses an unknown `schema` number and malformed or truncated JSON with a
//! typed [`ReadError`], and ignores fields it does not know. It does not recompute record
//! hashes: a newer writer may hash contract fields this reader does not know, and the file's
//! integrity is the signature's job.
pub mod records;
pub mod shape;

pub use records::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The schema this crate writes and reads. Adding a field does not change it; removing or
/// redefining one does.
pub const SCHEMA: u32 = 1;

/// The whole document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    pub schema: u32,
    pub hash: String,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    #[serde(default)]
    pub traits: Vec<Trait>,
    /// Inherent methods of the API's types, handles and values alike.
    #[serde(default)]
    pub handles: Vec<Function>,
    #[serde(default)]
    pub functions: Vec<Function>,
    #[serde(default)]
    pub macros: Vec<Macro>,
    #[serde(default)]
    pub types: Vec<Type>,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

/// Why a document was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// Not JSON, truncated, or a field of the wrong type.
    Json(String),
    /// A schema number this reader does not know.
    Schema(u32),
    /// Two records with one id.
    DuplicateId(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(f, "surface.json is not a valid surface: {error}"),
            Self::Schema(schema) => write!(
                f,
                "surface.json has schema {schema}; this tool reads schema {SCHEMA}"
            ),
            Self::DuplicateId(id) => write!(f, "surface.json lists {id} twice"),
        }
    }
}

impl std::error::Error for ReadError {}

/// The contract hash of a record: the first 128 bits, as 32 lowercase hex digits, of the SHA-256
/// of the contract's compact JSON (fields in declaration order, absent optional fields left out).
pub fn contract_hash<T: Serialize>(contract: &T) -> String {
    let text = serde_json::to_string(contract).expect("contracts serialize");
    shape::digest(&text)
}

/// The identifier a mod writes for an id: the last path segment, or the field or variant name.
/// A capability's is its key; its constant is in [`Surface::identifiers`].
pub fn leaf(id: &str) -> &str {
    let Some((kind, rest)) = id.split_once(':') else {
        return id;
    };
    match kind {
        "capability" | "hook" | "part" => rest,
        "type" => match rest.rsplit_once('.') {
            Some((_, member)) => member,
            None => rest.rsplit("::").next().unwrap_or(rest),
        },
        _ => rest.rsplit("::").next().unwrap_or(rest),
    }
}

macro_rules! sealed {
    ($($record:ident($contract:ty)),* $(,)?) => {$(
        impl $record {
            /// The record, with the hash of `contract`.
            pub fn new(id: impl Into<String>, doc: impl Into<String>, contract: $contract) -> Self {
                Self { id: id.into(), hash: contract_hash(&contract), doc: doc.into(), contract }
            }
        }
    )*};
}

sealed!(
    Method(MethodContract),
    Function(FunctionContract),
    Macro(MacroContract),
    Field(FieldContract),
    Variant(VariantContract),
    Capability(CapabilityContract),
);

impl Hook {
    pub fn new(
        id: impl Into<String>,
        doc: impl Into<String>,
        engine: HookEngine,
        counterparts: Counterparts,
        contract: HookContract,
    ) -> Self {
        Self {
            id: id.into(),
            hash: contract_hash(&contract),
            doc: doc.into(),
            engine,
            counterparts,
            contract,
        }
    }
}

impl Trait {
    pub fn new(
        id: impl Into<String>,
        doc: impl Into<String>,
        contract: TraitContract,
        methods: Vec<Method>,
    ) -> Self {
        Self {
            id: id.into(),
            hash: contract_hash(&contract),
            doc: doc.into(),
            contract,
            methods,
        }
    }
}

impl Type {
    pub fn new(
        id: impl Into<String>,
        doc: impl Into<String>,
        shape: String,
        contract: TypeContract,
        fields: Vec<Field>,
        variants: Vec<Variant>,
    ) -> Self {
        Self {
            id: id.into(),
            hash: contract_hash(&contract),
            doc: doc.into(),
            shape,
            contract,
            fields,
            variants,
        }
    }
}

impl Part {
    pub fn new(
        id: impl Into<String>,
        doc: impl Into<String>,
        hooks: Vec<String>,
        contract: PartContract,
    ) -> Self {
        Self {
            id: id.into(),
            hash: contract_hash(&contract),
            doc: doc.into(),
            hooks,
            contract,
        }
    }
}

impl Surface {
    /// The document: each list sorted by id, members in declaration order, and its hash.
    pub fn new(
        mut hooks: Vec<Hook>,
        mut traits: Vec<Trait>,
        mut handles: Vec<Function>,
        mut functions: Vec<Function>,
        mut macros: Vec<Macro>,
        mut types: Vec<Type>,
        mut parts: Vec<Part>,
        mut capabilities: Vec<Capability>,
    ) -> Self {
        hooks.sort_by(|a, b| a.id.cmp(&b.id));
        traits.sort_by(|a, b| a.id.cmp(&b.id));
        for item in &mut traits {
            item.methods.sort_by(|a, b| a.id.cmp(&b.id));
        }
        handles.sort_by(|a, b| a.id.cmp(&b.id));
        functions.sort_by(|a, b| a.id.cmp(&b.id));
        macros.sort_by(|a, b| a.id.cmp(&b.id));
        types.sort_by(|a, b| a.id.cmp(&b.id));
        parts.sort_by(|a, b| a.id.cmp(&b.id));
        capabilities.sort_by(|a, b| a.id.cmp(&b.id));
        let mut surface = Self {
            schema: SCHEMA,
            hash: String::new(),
            hooks,
            traits,
            handles,
            functions,
            macros,
            types,
            parts,
            capabilities,
        };
        surface.hash = surface.document_hash();
        surface
    }

    /// Every record and member record as (id, hash), in no promised order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        let hooks = self.hooks.iter().map(|r| (r.id.as_str(), r.hash.as_str()));
        let traits = self.traits.iter().flat_map(|r| {
            std::iter::once((r.id.as_str(), r.hash.as_str()))
                .chain(r.methods.iter().map(|m| (m.id.as_str(), m.hash.as_str())))
        });
        let functions = self
            .handles
            .iter()
            .chain(&self.functions)
            .map(|r| (r.id.as_str(), r.hash.as_str()));
        let macros = self.macros.iter().map(|r| (r.id.as_str(), r.hash.as_str()));
        let types = self.types.iter().flat_map(|r| {
            std::iter::once((r.id.as_str(), r.hash.as_str()))
                .chain(r.fields.iter().map(|f| (f.id.as_str(), f.hash.as_str())))
                .chain(r.variants.iter().map(|v| (v.id.as_str(), v.hash.as_str())))
        });
        let parts = self.parts.iter().map(|r| (r.id.as_str(), r.hash.as_str()));
        let capabilities = self
            .capabilities
            .iter()
            .map(|r| (r.id.as_str(), r.hash.as_str()));
        hooks
            .chain(traits)
            .chain(functions)
            .chain(macros)
            .chain(types)
            .chain(parts)
            .chain(capabilities)
    }

    /// The SHA-256 (first 128 bits, hex) of `<id> <hash>\n` for every entry, in byte order of id.
    pub fn document_hash(&self) -> String {
        let mut entries: Vec<(&str, &str)> = self.entries().collect();
        entries.sort_unstable();
        let mut text = String::new();
        for (id, hash) in entries {
            text.push_str(id);
            text.push(' ');
            text.push_str(hash);
            text.push('\n');
        }
        shape::digest(&text)
    }

    /// The identifiers a mod's source may contain for each id: its [`leaf`], and a capability
    /// key's constant.
    pub fn identifiers(&self) -> impl Iterator<Item = (&str, &str)> {
        let constants = self.capabilities.iter().filter_map(|c| {
            c.contract.constant.as_deref().map(|constant| {
                (
                    constant.rsplit("::").next().unwrap_or(constant),
                    c.id.as_str(),
                )
            })
        });
        self.entries()
            .map(|(id, _)| (leaf(id), id))
            .chain(constants)
    }

    /// `ids` with every API type or trait their signatures reference, transitively. A type's
    /// references are those of its fields, variants and alias target; a trait's, those of its
    /// supertraits. Unknown ids are kept and lead nowhere.
    pub fn closure<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
        let mut refs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for hook in &self.hooks {
            refs.insert(
                &hook.id,
                hook.contract.refs.iter().map(String::as_str).collect(),
            );
        }
        for item in &self.traits {
            refs.insert(
                &item.id,
                item.contract.refs.iter().map(String::as_str).collect(),
            );
            for method in &item.methods {
                refs.insert(
                    &method.id,
                    method.contract.refs.iter().map(String::as_str).collect(),
                );
            }
        }
        for function in self.handles.iter().chain(&self.functions) {
            refs.insert(
                &function.id,
                function.contract.refs.iter().map(String::as_str).collect(),
            );
        }
        for item in &self.types {
            let mut all: Vec<&str> = item.contract.refs.iter().map(String::as_str).collect();
            for field in &item.fields {
                let own: Vec<&str> = field.contract.refs.iter().map(String::as_str).collect();
                all.extend(&own);
                refs.insert(&field.id, own);
            }
            for variant in &item.variants {
                let own: Vec<&str> = variant.contract.refs.iter().map(String::as_str).collect();
                all.extend(&own);
                refs.insert(&variant.id, own);
            }
            refs.insert(&item.id, all);
        }
        for capability in &self.capabilities {
            refs.insert(
                &capability.id,
                capability
                    .contract
                    .refs
                    .iter()
                    .map(String::as_str)
                    .collect(),
            );
        }
        let mut out = BTreeSet::new();
        let mut pending: Vec<String> = ids.into_iter().map(str::to_owned).collect();
        while let Some(id) = pending.pop() {
            if let Some(next) = refs.get(id.as_str()) {
                pending.extend(
                    next.iter()
                        .filter(|r| !out.contains(**r))
                        .map(|r| (*r).to_owned()),
                );
            }
            out.insert(id);
        }
        out
    }

    /// Reads a document: refuses an unknown schema, malformed JSON and repeated ids.
    pub fn read(bytes: &[u8]) -> Result<Self, ReadError> {
        #[derive(Deserialize)]
        struct Header {
            schema: u32,
        }
        let header: Header =
            serde_json::from_slice(bytes).map_err(|e| ReadError::Json(e.to_string()))?;
        if header.schema != SCHEMA {
            return Err(ReadError::Schema(header.schema));
        }
        let surface: Self =
            serde_json::from_slice(bytes).map_err(|e| ReadError::Json(e.to_string()))?;
        let mut seen = BTreeSet::new();
        for (id, _) in surface.entries() {
            if !seen.insert(id) {
                return Err(ReadError::DuplicateId(id.to_owned()));
            }
        }
        Ok(surface)
    }

    /// The document as written: pretty JSON, ending in a newline.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a surface serializes");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests;
