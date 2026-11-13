//! The record kinds of `surface.json`. Each record is `id`, `hash`, the fields its hash leaves
//! out (`doc`, and `engine`, `counterparts` or `shape` where the kind has one) and its contract,
//! written flat beside them. [`crate::contract_hash`] of the contract is the record's `hash`.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn is_false(value: &bool) -> bool {
    !value
}

/// Which side of the connection something runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Server,
    Client,
    Both,
}

/// A named value in a signature: a hook argument or output, or a variant's field (named by its
/// position when it has no name).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argument {
    pub name: String,
    /// Normalised Rust, paths relative to `mod_api`.
    #[serde(rename = "type")]
    pub ty: String,
}

/// What a hook's listeners may do with its event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookKind {
    Listen,
    Cancel,
    Collect,
    Replace,
}

/// A scoped hook: a Part method, called on the Parts of what the argument `key` names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub kind: String,
    pub key: String,
}

/// Where a hook fires: information only, outside the hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookEngine {
    /// The game function that fires it.
    pub target: String,
    /// The load-phase context it takes instead of a host, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// The names a hook has in other mod APIs: information only, outside the hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counterparts {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub neoforge: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fabric: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paper: Vec<String>,
}

impl Counterparts {
    fn is_empty(&self) -> bool {
        self.neoforge.is_empty() && self.fabric.is_empty() && self.paper.is_empty()
    }
}

/// A hook's contract: everything a listener relies on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookContract {
    pub version: u64,
    pub side: Side,
    pub contract: HookKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellable: Option<bool>,
    /// The host a listener gets when its receiver does not say which (`read`, `level`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// The argument naming the one mod whose subscription alone is told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// The output also names the mod that answered or pushed it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub answered_by_mod: bool,
    pub arguments: Vec<Argument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Argument>,
    /// The ids of the API types and traits its arguments and output name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `hook:<name>`: a moment the game tells mods about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hook {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(default)]
    pub engine: HookEngine,
    #[serde(default, skip_serializing_if = "Counterparts::is_empty")]
    pub counterparts: Counterparts,
    #[serde(flatten)]
    pub contract: HookContract,
}

/// Whether a callable is available to WebAssembly mods: `true`, or the reason it is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wasm {
    Yes,
    No(String),
}

impl Serialize for Wasm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Yes => serializer.serialize_bool(true),
            Self::No(why) => serializer.serialize_str(why),
        }
    }
}

impl<'de> Deserialize<'de> for Wasm {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Form {
            Flag(bool),
            Reason(String),
        }
        match Form::deserialize(deserializer)? {
            Form::Flag(true) => Ok(Self::Yes),
            Form::Flag(false) => Err(serde::de::Error::custom(
                "wasm is true or the reason it is not",
            )),
            Form::Reason(why) => Ok(Self::No(why)),
        }
    }
}

/// What a mod does with a trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Mods implement it; the game calls it.
    Implement,
    /// The game implements it; mods call it.
    Call,
    /// It does not cross the boundary: a mod uses it inside its own code.
    None,
}

/// A trait method a mod must write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Required {
    pub name: String,
    pub signature: String,
}

/// A trait's own contract. Its methods are records of their own; only the required methods of
/// a trait mods implement are part of it, because a new one breaks every implementation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitContract {
    pub role: Role,
    pub side: Side,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generics: String,
    /// Supertraits, normalised.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<String>,
    /// With role `implement`: the methods with no default, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<Required>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `trait:<path>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: TraitContract,
    #[serde(default)]
    pub methods: Vec<Method>,
}

/// A trait method's contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodContract {
    /// `fn name<…>(…) -> …`, normalised, lifetimes kept.
    pub signature: String,
    /// It has no default body.
    pub required: bool,
    pub wasm: Wasm,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `method:<Trait path>::<name>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Method {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: MethodContract,
}

/// A free function's or inherent method's contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionContract {
    pub signature: String,
    pub wasm: Wasm,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `fn:<path>` (a free function) or `handle:<Type path>::<name>` (an inherent method).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: FunctionContract,
}

/// How a macro is invoked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroKind {
    /// `#[name]`.
    Attribute,
    /// `#[derive(Name)]`.
    Derive,
    /// `name!(…)`, a procedural macro.
    Function,
    /// `name!(…)`, a `macro_rules!` macro.
    Declarative,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroContract {
    pub kind: MacroKind,
    /// A derive's helper attributes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
}

/// `macro:<path>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: MacroContract,
}

/// What a type is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Struct,
    TupleStruct,
    UnitStruct,
    Enum,
    Union,
    /// `type Name = …;`
    Alias,
}

/// A type's own contract. Its fields and variants are records of their own; the member set is
/// part of it only where a new member breaks users: the fields of a `constructible` struct and
/// the variants of an exhaustive enum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeContract {
    pub kind: TypeKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generics: String,
    /// The traits of its `#[derive(…)]` lists, by last path segment, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derives: Vec<String>,
    /// It derives `Wire`.
    pub wire: bool,
    /// A mod can build it with a literal: every field is public and it is not
    /// `#[non_exhaustive]`. For an enum: it is not `#[non_exhaustive]`.
    pub constructible: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub non_exhaustive: bool,
    /// An alias's target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// For a constructible struct its fields (`name: Type`), for an exhaustive enum its variants
    /// (`Name`, `Name(A, B)`, `Name { a: A }`), in declaration order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    /// An alias's references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `type:<path>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Type {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    /// Its wire layout hash ([`crate::shape`]), outside the contract hash.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shape: String,
    #[serde(flatten)]
    pub contract: TypeContract,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldContract {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `type:<path>.<field>`: a public field (a tuple struct's by position).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: FieldContract,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantContract {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Argument>,
    /// Its fields have names (`{ … }`), not positions.
    #[serde(default, skip_serializing_if = "is_false")]
    pub named: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `type:<path>.<Variant>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: VariantContract,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartContract {
    /// The id of the trait a Part of this kind implements.
    #[serde(rename = "trait")]
    pub trait_id: String,
}

/// `part:<kind>`: a kind of object mods attach Parts to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    /// The scoped hooks that call this kind's Parts: records of their own, outside the hash.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
    #[serde(flatten)]
    pub contract: PartContract,
}

/// Where a capability comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// A `capabilities` entry of `horizon-mod.toml`.
    Manifest,
    /// A capability key a block answers.
    Block,
    /// A capability key an item answers.
    Item,
    /// A capability key an entity answers.
    Entity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityContract {
    pub kind: CapabilityKind,
    /// A key's constant, as a mod names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constant: Option<String>,
    /// A key's handler trait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<String>,
    /// An entity key's lookup context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// `capability:<key>`: a manifest capability (`registries`) or a capability key
/// (`horizon:fluids`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(flatten)]
    pub contract: CapabilityContract,
}
