//! How a game internal crosses to a wasm mod (#589 W): one rule for both sides, the mod's generated
//! view and the game's generated table, read from the same `internals.json` signatures.
//!
//! - A **copy** is plain data all the way down, at most [`LEAVES`] scalars: it crosses as a value.
//! - A **handle** has fields or elements a mod walks into: it crosses as a node of a path. Its
//!   parts that are opaque are just not reachable.
//! - An **opaque** type crosses neither way: trait objects, closures, cells and locks, channels,
//!   GPU and window objects, raw pointers, generic types, and types this game doesn't list.
//!
//! Each written type resolves to a [`Shape`], whose types are spelled from the game crate's root,
//! so the game's generated code names exactly the type the signature means.
use crate::internals::Internal;
use std::collections::HashMap;
use syn::{GenericArgument, PathArguments, Type};

/// The most scalars a copy holds; a bigger plain type crosses as a handle.
pub const LEAVES: usize = 16;

/// How one type crosses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    /// A value of this many scalars.
    Copy(usize),
    /// A node a mod reads into, field by field or element by element.
    Handle,
    /// Nothing crosses.
    Opaque,
}

/// A scalar's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scalar {
    Bool,
    Char,
    /// A signed integer, by its Rust name.
    Signed(&'static str),
    Unsigned(&'static str),
    Float(&'static str),
    /// `str`: read only.
    Str,
    String,
}

/// A wrapper read through: `Box` owns its value; `Arc`, `Rc` and `Cow` share it, so it is read
/// only through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrapper {
    Box,
    Arc,
    Rc,
    Cow,
}

/// A std collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collection {
    Vec,
    VecDeque,
    HashSet,
    BTreeSet,
    HashMap,
    BTreeMap,
}

/// What a written type is, resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Scalar(Scalar),
    Unit,
    Ref {
        mutable: bool,
        to: Box<Shape>,
    },
    Tuple(Vec<Shape>),
    Array(Box<Shape>, usize),
    Slice(Box<Shape>),
    Option(Box<Shape>),
    Wrapped(Wrapper, Box<Shape>),
    /// A collection, with its written arguments (a map's key and value, and any hasher, as types
    /// from the crate's root).
    Collection(Collection, Vec<Shape>),
    /// A struct or enum this game lists, by path, with its count of lifetime parameters.
    Named(String, usize),
    /// A type that never crosses, such as a hasher argument: spelled, never walked.
    Opaque(Option<String>),
}

/// Scalars by name.
const SCALARS: &[(&str, Scalar)] = &[
    ("bool", Scalar::Bool),
    ("char", Scalar::Char),
    ("i8", Scalar::Signed("i8")),
    ("i16", Scalar::Signed("i16")),
    ("i32", Scalar::Signed("i32")),
    ("i64", Scalar::Signed("i64")),
    ("isize", Scalar::Signed("isize")),
    ("u8", Scalar::Unsigned("u8")),
    ("u16", Scalar::Unsigned("u16")),
    ("u32", Scalar::Unsigned("u32")),
    ("u64", Scalar::Unsigned("u64")),
    ("usize", Scalar::Unsigned("usize")),
    ("f32", Scalar::Float("f32")),
    ("f64", Scalar::Float("f64")),
    ("str", Scalar::Str),
    ("String", Scalar::String),
];

const WRAPPERS: &[(&str, Wrapper)] = &[
    ("Box", Wrapper::Box),
    ("Arc", Wrapper::Arc),
    ("Rc", Wrapper::Rc),
    ("Cow", Wrapper::Cow),
];

const COLLECTIONS: &[(&str, Collection)] = &[
    ("Vec", Collection::Vec),
    ("VecDeque", Collection::VecDeque),
    ("HashSet", Collection::HashSet),
    ("BTreeSet", Collection::BTreeSet),
    ("HashMap", Collection::HashMap),
    ("BTreeMap", Collection::BTreeMap),
];

/// Crates whose types never cross: the GPU, the window, input devices, audio.
const OPAQUE_CRATES: &[&str] = &["wgpu", "winit", "gilrs", "cpal", "rodio", "glyphon"];

/// The game's crate: its paths are spelled from `crate`.
const GAME: &str = "horizon_client";

/// A listed struct or enum.
pub struct Named {
    pub item: syn::Item,
    /// The module it is declared in, where its fields' types are written.
    pub module: String,
    pub lifetimes: usize,
    /// Generic over a type or a const: no one layout, so opaque.
    pub generic: bool,
}

/// A field of a listed type, in the numbering the table and the view share: declaration order,
/// across an enum's cases.
pub struct Field<'r> {
    /// The case it is in, for an enum: its index and name.
    pub case: Option<(u32, &'r syn::Ident)>,
    /// Its name, or its index in a tuple struct or case.
    pub member: syn::Member,
    pub ty: &'r Type,
}

/// The rule over one game's internals.
pub struct Rule {
    named: HashMap<String, Named>,
    /// Each type name's paths, for a name written without its module.
    by_name: HashMap<String, Vec<String>>,
    memo: std::cell::RefCell<HashMap<String, Crossing>>,
}

/// `a::b::C<'_>` as `a::b::C`.
pub fn bare(path: &str) -> String {
    path.split("::")
        .map(|segment| segment.split('<').next().unwrap_or(segment))
        .collect::<Vec<_>>()
        .join("::")
}

impl Rule {
    /// The rule over `internals`: every struct and enum they list, under no `cfg`.
    pub fn new(internals: &[Internal]) -> Self {
        let mut named = HashMap::new();
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        for internal in internals {
            if !internal.cfg.is_empty() || !matches!(internal.kind, "struct" | "enum") {
                continue;
            }
            let Ok(item) = syn::parse_str::<syn::Item>(&internal.signature) else {
                continue;
            };
            let generics = match &item {
                syn::Item::Struct(item) => &item.generics,
                syn::Item::Enum(item) => &item.generics,
                _ => continue,
            };
            let lifetimes = generics.lifetimes().count();
            let generic = generics.params.len() > lifetimes;
            let path = bare(&internal.path);
            let (module, name) = path.rsplit_once("::").unwrap_or(("", &path));
            by_name
                .entry(name.to_owned())
                .or_default()
                .push(path.clone());
            named.insert(
                path.clone(),
                Named {
                    module: module.to_owned(),
                    item,
                    lifetimes,
                    generic,
                },
            );
        }
        Self {
            named,
            by_name,
            memo: Default::default(),
        }
    }

    /// The listed type at `path`.
    pub fn named(&self, path: &str) -> Option<&Named> {
        self.named.get(path)
    }

    /// The fields of the listed type at `path`, numbered.
    pub fn fields<'r>(&'r self, path: &str) -> Vec<Field<'r>> {
        let fields = |case, fields: &'r syn::Fields| {
            fields.iter().enumerate().map(move |(i, field)| Field {
                case,
                member: field.ident.clone().map_or(
                    syn::Member::Unnamed(syn::Index::from(i)),
                    syn::Member::Named,
                ),
                ty: &field.ty,
            })
        };
        match self.named.get(path).map(|named| &named.item) {
            Some(syn::Item::Struct(item)) => fields(None, &item.fields).collect(),
            Some(syn::Item::Enum(item)) => item
                .variants
                .iter()
                .enumerate()
                .flat_map(|(i, v)| fields(Some((i as u32, &v.ident)), &v.fields))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The listed type `written` names in module `module`: its path as written if listed, else the
    /// one of its name in that module or above it, else the only one of its name.
    fn resolve(&self, module: &str, written: &syn::Path) -> Option<String> {
        let spelled = written
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let spelled = spelled
            .strip_prefix("crate::")
            .map_or(spelled.clone(), |rest| format!("{GAME}::{rest}"));
        if self.named.contains_key(&spelled) {
            return Some(spelled);
        }
        let candidates = self
            .by_name
            .get(&written.segments.last()?.ident.to_string())?;
        let mut scope = module;
        loop {
            let here = format!("{scope}::{spelled}");
            if self.named.contains_key(&here) {
                return Some(here);
            }
            match scope.rsplit_once("::") {
                Some((up, _)) => scope = up,
                None => break,
            }
        }
        (candidates.len() == 1).then(|| candidates[0].clone())
    }

    /// What `ty`, written in module `module` (where `Self` is `this`), is.
    pub fn shape(&self, module: &str, this: Option<&str>, ty: &Type) -> Shape {
        let opaque = Shape::Opaque(None);
        match ty {
            Type::Reference(r) => Shape::Ref {
                mutable: r.mutability.is_some(),
                to: Box::new(self.shape(module, this, &r.elem)),
            },
            Type::Paren(p) => self.shape(module, this, &p.elem),
            Type::Group(g) => self.shape(module, this, &g.elem),
            Type::Tuple(t) if t.elems.is_empty() => Shape::Unit,
            Type::Tuple(t) => Shape::Tuple(
                t.elems
                    .iter()
                    .map(|e| self.shape(module, this, e))
                    .collect(),
            ),
            Type::Array(a) => match &a.len {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => n.base10_parse().map_or(opaque, |n| {
                    Shape::Array(Box::new(self.shape(module, this, &a.elem)), n)
                }),
                _ => opaque,
            },
            Type::Slice(s) => Shape::Slice(Box::new(self.shape(module, this, &s.elem))),
            Type::Path(p) if p.qself.is_none() => self.path(module, this, &p.path),
            _ => opaque,
        }
    }

    fn path(&self, module: &str, this: Option<&str>, path: &syn::Path) -> Shape {
        let opaque = Shape::Opaque(None);
        let Some(last) = path.segments.last() else {
            return opaque;
        };
        let name = last.ident.to_string();
        let args: Vec<Shape> = match &last.arguments {
            PathArguments::AngleBracketed(a) => a
                .args
                .iter()
                .filter_map(|a| match a {
                    GenericArgument::Type(t) => Some(self.shape(module, this, t)),
                    _ => None,
                })
                .collect(),
            PathArguments::None => Vec::new(),
            PathArguments::Parenthesized(_) => return opaque,
        };
        let first = path.segments[0].ident.to_string();
        if OPAQUE_CRATES.contains(&first.as_str()) {
            return opaque;
        }
        if name == "Self" && path.segments.len() == 1 {
            return this.map_or(opaque, |this| self.named_shape(this));
        }
        let single = |args: Vec<Shape>| -> Option<Box<Shape>> {
            let [inner] = <[Shape; 1]>::try_from(args).ok()?;
            Some(Box::new(inner))
        };
        if let Some((_, scalar)) = SCALARS.iter().find(|(n, _)| *n == name) {
            return if args.is_empty() {
                Shape::Scalar(*scalar)
            } else {
                opaque
            };
        }
        if name == "Option" {
            return single(args).map_or(opaque, Shape::Option);
        }
        if let Some((_, wrapper)) = WRAPPERS.iter().find(|(n, _)| *n == name) {
            return single(args).map_or(opaque, |inner| Shape::Wrapped(*wrapper, inner));
        }
        if let Some((_, collection)) = COLLECTIONS.iter().find(|(n, _)| *n == name) {
            let keyed = matches!(collection, Collection::HashMap | Collection::BTreeMap);
            let wanted = if keyed { 2 } else { 1 };
            // A hasher argument is spelled as it is resolved, and never walked.
            return if args.len() < wanted {
                opaque
            } else {
                Shape::Collection(*collection, args)
            };
        }
        match self.resolve(module, path) {
            Some(found) => self.named_shape(&found),
            None => opaque,
        }
    }

    fn named_shape(&self, path: &str) -> Shape {
        match self.named.get(path) {
            Some(named) if !named.generic => Shape::Named(path.to_owned(), named.lifetimes),
            _ => Shape::Opaque(None),
        }
    }

    /// How `ty`, written in module `module`, crosses.
    pub fn crossing(&self, module: &str, ty: &Type) -> Crossing {
        self.of(&self.shape(module, None, ty), &mut Vec::new())
    }

    /// How `shape` crosses.
    pub fn crossing_of(&self, shape: &Shape) -> Crossing {
        self.of(shape, &mut Vec::new())
    }

    fn of(&self, shape: &Shape, open: &mut Vec<String>) -> Crossing {
        match shape {
            Shape::Scalar(_) | Shape::Unit => Crossing::Copy(1),
            Shape::Ref { to, .. } | Shape::Wrapped(_, to) => self.of(to, open),
            Shape::Tuple(parts) => self.all(parts.iter(), open),
            Shape::Array(elem, n) => match self.of(elem, open) {
                Crossing::Copy(leaves) if leaves * n <= LEAVES => Crossing::Copy(leaves * n),
                Crossing::Opaque => Crossing::Opaque,
                _ => Crossing::Handle,
            },
            Shape::Option(inner) => match self.of(inner, open) {
                Crossing::Copy(leaves) if leaves < LEAVES => Crossing::Copy(leaves + 1),
                Crossing::Opaque => Crossing::Opaque,
                _ => Crossing::Handle,
            },
            Shape::Slice(elem) => match self.of(elem, open) {
                Crossing::Opaque => Crossing::Opaque,
                _ => Crossing::Handle,
            },
            Shape::Collection(collection, args) => {
                let walked = match collection {
                    Collection::HashMap | Collection::BTreeMap => &args[..2],
                    _ => &args[..1],
                };
                if walked.iter().any(|a| self.of(a, open) == Crossing::Opaque) {
                    Crossing::Opaque
                } else {
                    Crossing::Handle
                }
            }
            Shape::Named(path, _) => self.named_crossing(path, open),
            Shape::Opaque(_) => Crossing::Opaque,
        }
    }

    /// How the listed type at `path` crosses, each once.
    fn named_crossing(&self, path: &str, open: &mut Vec<String>) -> Crossing {
        if let Some(known) = self.memo.borrow().get(path) {
            return *known;
        }
        // A type that holds itself, through a box or a list, is walked, not copied.
        if open.iter().any(|p| p == path) {
            return Crossing::Handle;
        }
        let named = &self.named[path];
        open.push(path.to_owned());
        let parts: Vec<Shape> = self
            .fields(path)
            .iter()
            .map(|field| self.shape(&named.module, Some(path), field.ty))
            .collect();
        let crossing = self.all(parts.iter(), open);
        open.pop();
        self.memo.borrow_mut().insert(path.to_owned(), crossing);
        crossing
    }

    /// A type made of `parts`: a copy when each is and their leaves fit, else a handle, whose parts
    /// that can't cross simply can't be reached. A type with no parts is one leaf (`()`, a unit
    /// case).
    fn all<'s>(&self, parts: impl Iterator<Item = &'s Shape>, open: &mut Vec<String>) -> Crossing {
        let mut leaves = 0;
        let mut copy = true;
        let mut any = false;
        for part in parts {
            any = true;
            match self.of(part, open) {
                Crossing::Copy(n) => leaves += n,
                Crossing::Handle | Crossing::Opaque => copy = false,
            }
        }
        match (any, copy) {
            (false, _) => Crossing::Copy(1),
            (true, true) if leaves <= LEAVES => Crossing::Copy(leaves),
            _ => Crossing::Handle,
        }
    }

    /// Every listed type's path.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.named.keys().map(String::as_str)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn internal(path: &str, kind: &'static str, signature: &str) -> Internal {
        Internal {
            path: path.into(),
            kind,
            cfg: String::new(),
            signature: signature.into(),
            hash: String::new(),
        }
    }

    fn rule() -> Rule {
        Rule::new(&[
            internal(
                "g::Pos",
                "struct",
                "struct Pos { x : i32 , y : i32 , z : i32 , }",
            ),
            internal(
                "g::Mode",
                "enum",
                "enum Mode { Survival , Creative , Custom (u8) , }",
            ),
            internal(
                "g::Player",
                "struct",
                "struct Player { pos : Pos , mode : Mode , name : String , }",
            ),
            internal(
                "g::World",
                "struct",
                "struct World { players : Vec < Player > , spawn : Pos , }",
            ),
            internal(
                "g::Game",
                "struct",
                "struct Game < 'a > { world : World , ui : RefCell < u8 > , }",
            ),
            internal("g::Big", "struct", "struct Big { a : [u64 ; 17] , }"),
            internal("g::Tree", "struct", "struct Tree { kids : Vec < Tree > , }"),
            internal("g::Cache", "struct", "struct Cache < T > { item : T , }"),
            internal("g::a::Pos", "struct", "struct Pos { only : bool , }"),
        ])
    }

    fn of(rule: &Rule, module: &str, ty: &str) -> Crossing {
        rule.crossing(module, &syn::parse_str(ty).unwrap())
    }

    #[test]
    fn plain_data_copies_up_to_its_leaves_and_the_rest_is_walked_or_opaque() {
        let rule = rule();
        assert_eq!(of(&rule, "g", "[i32; 3]"), Crossing::Copy(3));
        assert_eq!(of(&rule, "g", "Pos"), Crossing::Copy(3));
        assert_eq!(of(&rule, "g", "Option<Pos>"), Crossing::Copy(4));
        assert_eq!(of(&rule, "g", "(u8, &str)"), Crossing::Copy(2));
        assert_eq!(of(&rule, "g", "Mode"), Crossing::Copy(1));
        // A player is plain data: a copy, its name one leaf.
        assert_eq!(of(&rule, "g", "Player"), Crossing::Copy(5));
        assert_eq!(of(&rule, "g", "World"), Crossing::Handle);
        assert_eq!(of(&rule, "g", "Big"), Crossing::Handle);
        assert_eq!(of(&rule, "g", "Tree"), Crossing::Handle);
        assert_eq!(of(&rule, "g", "&mut Vec<Pos>"), Crossing::Handle);
        // A cell in it makes `Game` no copy, but a handle: its other fields are reached one by
        // one, and the cell can't be.
        assert_eq!(of(&rule, "g", "Game<'_>"), Crossing::Handle);
        assert_eq!(of(&rule, "g", "(u8, RefCell<u8>)"), Crossing::Handle);
        assert_eq!(of(&rule, "g", "Cache<u8>"), Crossing::Opaque);
        for opaque in [
            "Box<dyn Fn()>",
            "impl Iterator<Item = u8>",
            "*const u8",
            "wgpu::Device",
            "Mutex<u8>",
            "Unlisted",
        ] {
            assert_eq!(of(&rule, "g", opaque), Crossing::Opaque, "{opaque}");
        }
    }

    #[test]
    fn a_name_resolves_in_its_own_module_first() {
        let rule = rule();
        assert_eq!(of(&rule, "g::a", "Pos"), Crossing::Copy(1));
        assert_eq!(of(&rule, "g", "Pos"), Crossing::Copy(3));
        assert_eq!(of(&rule, "g::b", "a::Pos"), Crossing::Copy(1));
    }
}
