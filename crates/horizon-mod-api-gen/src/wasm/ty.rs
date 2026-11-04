//! Rust types of the mod API's signatures, resolved to the shapes that cross the WebAssembly
//! boundary.
use super::index::{Found, Index, ItemId};
use heck::{ToKebabCase, ToUpperCamelCase};
use quote::ToTokens;
use std::collections::HashMap;
use syn::{Fields, GenericArgument, Item, PathArguments, Type};

/// A Rust type as it crosses: the WIT type it lowers to and how its values convert.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ty {
    Unit,
    /// A primitive: its Rust and WIT names.
    Prim(&'static str, &'static str),
    /// `u128`/`i128`, as two halves.
    Wide(&'static str),
    /// `std::time::Duration`, as nanoseconds.
    Duration,
    /// `String`, `&str`, `Box<str>`, `Arc<str>`: owned once lifted, except `&'static str`.
    Str(Str),
    List(Box<Ty>, List),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    Array(Box<Ty>, usize),
    /// `&T` or `&mut T`.
    Ref(Box<Ty>, bool),
    Boxed(Box<Ty>),
    Named(usize),
    /// `&dyn Trait` or `&mut dyn Trait` (the reference is part of it): a frame resource.
    Dyn(String, bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Str {
    Owned,
    Borrowed,
    Static,
    Boxed,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum List {
    Vec,
    /// A map, as its container names it, crossing as a list of pairs.
    Map(&'static str),
    Slice,
    Boxed,
    Shared,
}

/// A named type's definition.
#[derive(Debug)]
pub(crate) struct Def {
    /// Its path as the generated glue names it (`mod_api::NativeBlockClick`).
    pub(super) rust: String,
    /// Its WIT name, also the bindings' type name in upper camel case.
    pub(super) wit: String,
    /// Its lifetime parameters: it borrows when there are any.
    pub(super) lifetimes: usize,
    pub(super) kind: Kind,
}

#[derive(Debug)]
pub(crate) enum Kind {
    Record(Vec<Field>),
    /// A tuple struct with public fields.
    TupleRecord(Vec<Ty>),
    Enum(Vec<String>),
    /// A unit struct.
    Unit,
    Variant(Vec<Case>),
    /// Crosses as its form, declared on the type with `#[wasm(via = Form)]`.
    Via(Ty),
    /// Crosses as JSON through its own serde derives.
    Json,
    /// It can't cross; why.
    Opaque(String),
}

#[derive(Debug)]
pub(crate) struct Field {
    pub(super) rust: String,
    pub(super) wit: String,
    pub(super) ty: Ty,
}

#[derive(Debug)]
pub(crate) struct Case {
    pub(super) rust: String,
    pub(super) wit: String,
    pub(super) payload: Payload,
}

#[derive(Debug)]
pub(crate) enum Payload {
    None,
    Tuple(Vec<Ty>),
    /// Named fields, carried by a generated record named `record`.
    Record(String, Vec<Field>),
}

/// The resolved types, by definition.
pub(crate) struct Types<'a> {
    pub(super) index: Index<'a>,
    pub(super) defs: Vec<Def>,
    /// Why each def can't cross, if it can't (`wit::refuse`).
    pub(super) refused: Vec<Option<String>>,
    by_item: HashMap<ItemId, usize>,
    /// Names given to definitions so far, to keep WIT names unique.
    wit_names: HashMap<String, usize>,
    /// The `mod_api` root module, where re-exported names are looked up.
    api_root: usize,
    /// The forms of the types `mod_api` re-exports with `#[wasm(via = Form)]` on the `pub use`.
    exported_via: HashMap<ItemId, Type>,
    /// The same for re-exports of types the index can't read (made by a macro), by name.
    exported_via_names: HashMap<String, Type>,
}

impl<'a> Types<'a> {
    pub(super) fn new(mut index: Index<'a>) -> Result<Self, String> {
        let api_root = index.module("mod_api", &[])?;
        let mut exported_via = HashMap::new();
        let mut exported_via_names = HashMap::new();
        let uses: Vec<(String, Type)> = index.modules[api_root]
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Use(item) => Some((use_name(&item.tree)?, via(&item.attrs)?)),
                _ => None,
            })
            .collect();
        for (name, form) in uses {
            match index.resolve(api_root, &name) {
                Ok(Found::Item(id)) => {
                    exported_via.insert(id, form);
                }
                _ => {
                    exported_via_names.insert(name, form);
                }
            }
        }
        Ok(Self {
            index,
            exported_via,
            exported_via_names,
            defs: Vec::new(),
            refused: Vec::new(),
            by_item: HashMap::new(),
            wit_names: HashMap::new(),
            api_root,
        })
    }

    /// `ty`, written in `module`, with `subst` giving the generic parameters in scope.
    pub(super) fn ty(
        &mut self,
        module: usize,
        ty: &Type,
        subst: &[(String, Ty)],
    ) -> Result<Ty, String> {
        Ok(match ty {
            Type::Paren(ty) => self.ty(module, &ty.elem, subst)?,
            Type::Group(ty) => self.ty(module, &ty.elem, subst)?,
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ty::Unit,
            Type::Tuple(tuple) => Ty::Tuple(
                tuple
                    .elems
                    .iter()
                    .map(|ty| self.ty(module, ty, subst))
                    .collect::<Result<_, _>>()?,
            ),
            Type::Array(array) => {
                let len = match &array.len {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(len),
                        ..
                    }) => len.base10_parse().map_err(|e| e.to_string())?,
                    len => return Err(format!("array length {}", len.to_token_stream())),
                };
                Ty::Array(Box::new(self.ty(module, &array.elem, subst)?), len)
            }
            Type::Slice(slice) => {
                Ty::List(Box::new(self.ty(module, &slice.elem, subst)?), List::Slice)
            }
            Type::Reference(reference) => {
                let mutable = reference.mutability.is_some();
                let is_static = reference
                    .lifetime
                    .as_ref()
                    .is_some_and(|lifetime| lifetime.ident == "static");
                match &*reference.elem {
                    Type::TraitObject(object) => Ty::Dyn(trait_name(object)?, mutable),
                    Type::Path(path) if path.path.is_ident("str") => Ty::Str(if is_static {
                        Str::Static
                    } else {
                        Str::Borrowed
                    }),
                    Type::Slice(slice) if !mutable => {
                        Ty::List(Box::new(self.ty(module, &slice.elem, subst)?), List::Slice)
                    }
                    elem => Ty::Ref(Box::new(self.ty(module, elem, subst)?), mutable),
                }
            }
            Type::Path(path) if path.qself.is_none() => self.path(module, &path.path, subst)?,
            ty => return Err(format!("unsupported type {}", ty.to_token_stream())),
        })
    }

    fn path(
        &mut self,
        module: usize,
        path: &syn::Path,
        subst: &[(String, Ty)],
    ) -> Result<Ty, String> {
        let last = path.segments.last().ok_or("empty path")?;
        let name = last.ident.to_string();
        if path.segments.len() == 1
            && let Some((_, ty)) = subst.iter().find(|(param, _)| *param == name)
        {
            return Ok(ty.clone());
        }
        let args = match &last.arguments {
            PathArguments::AngleBracketed(args) => args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    GenericArgument::Type(ty) => Some(self.ty(module, ty, subst)),
                    _ => None,
                })
                .collect::<Result<Vec<_>, _>>()?,
            PathArguments::None => Vec::new(),
            PathArguments::Parenthesized(_) => return Err(format!("closure type {name}")),
        };
        let one = |args: &[Ty]| -> Result<Box<Ty>, String> {
            args.first()
                .cloned()
                .map(Box::new)
                .ok_or_else(|| format!("{name} needs a type argument"))
        };
        if let Some(prim) = primitive(&name) {
            return Ok(prim);
        }
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        // Prelude and std types, found by name whatever path reaches them.
        let found = self.index.resolve_path(module, &segments);
        let external = match &found {
            Ok(Found::External(path)) => Some(path.last().cloned().unwrap_or_default()),
            Err(_) => Some(name.clone()),
            Ok(Found::Item(_)) => None,
        };
        if let Some(std) = external {
            if let Some(form) = self.exported_via_names.get(&std).cloned() {
                let wit = std.to_kebab_case();
                if let Some(&def) = self.wit_names.get(&wit) {
                    return Ok(Ty::Named(def));
                }
                let def = self.push(Def {
                    rust: format!("mod_api::{std}"),
                    wit,
                    lifetimes: 0,
                    kind: Kind::Opaque("recursive".into()),
                });
                self.defs[def].kind = Kind::Via(self.ty(self.api_root, &form, &[])?);
                return Ok(Ty::Named(def));
            }
            return Ok(match std.as_str() {
                "String" => Ty::Str(Str::Owned),
                "Option" => Ty::Option(one(&args)?),
                "Vec" => Ty::List(one(&args)?, List::Vec),
                "Result" => {
                    let err = args.get(1).cloned().ok_or("Result needs an error type")?;
                    Ty::Result(one(&args)?, Box::new(err))
                }
                "Box" => match *one(&args)? {
                    Ty::Str(_) => Ty::Str(Str::Boxed),
                    Ty::List(item, _) => Ty::List(item, List::Boxed),
                    ty => Ty::Boxed(Box::new(ty)),
                },
                "Arc" | "Rc" => match *one(&args)? {
                    Ty::Str(_) => Ty::Str(Str::Shared),
                    Ty::List(item, _) => Ty::List(item, List::Shared),
                    ty => return Err(format!("{std}<{ty:?}>")),
                },
                "str" => Ty::Str(Str::Borrowed),
                "HashMap" | "BTreeMap" => {
                    let value = args.get(1).cloned().ok_or("map needs a value type")?;
                    let map = if std == "HashMap" {
                        "std::collections::HashMap"
                    } else {
                        "std::collections::BTreeMap"
                    };
                    Ty::List(
                        Box::new(Ty::Tuple(vec![*one(&args)?, value])),
                        List::Map(map),
                    )
                }
                "Value" | "Map" => Ty::Named(self.json(&segments)?),
                "Duration" => Ty::Duration,
                _ => return Err(format!("unsupported type {}", segments.join("::"))),
            });
        }
        let Ok(Found::Item(id)) = found else {
            unreachable!()
        };
        self.item(id, &name, args)
    }

    /// A JSON value crossing as its serde form.
    fn json(&mut self, segments: &[String]) -> Result<usize, String> {
        let name = segments.last().unwrap();
        let rust = match name.as_str() {
            "Value" => "serde_json::Value",
            _ => "serde_json::Map<String, serde_json::Value>",
        };
        if let Some(&id) = self
            .wit_names
            .get(&format!("json-{}", name.to_kebab_case()))
        {
            return Ok(id);
        }
        Ok(self.push(Def {
            rust: rust.into(),
            wit: format!("json-{}", name.to_kebab_case()),
            lifetimes: 0,
            kind: Kind::Json,
        }))
    }

    fn push(&mut self, def: Def) -> usize {
        let id = self.defs.len();
        self.wit_names.insert(def.wit.clone(), id);
        self.defs.push(def);
        id
    }

    /// The named item `id`, used with type arguments `args`.
    fn item(&mut self, id: ItemId, name: &str, args: Vec<Ty>) -> Result<Ty, String> {
        let item = self.index.item(id).clone();
        let form = match self.exported_via.get(&id) {
            Some(form) => Some((self.api_root, form.clone())),
            None => via(attrs(&item)).map(|form| (id.module, form)),
        };
        if let Some((module, form)) = form {
            if let Some(&def) = self.by_item.get(&id) {
                return Ok(Ty::Named(def));
            }
            let rust = self.public_path(id, name);
            let wit = self.unique_wit(id, name);
            let def = self.push(Def {
                rust,
                wit,
                lifetimes: 0,
                kind: Kind::Opaque("recursive".into()),
            });
            self.by_item.insert(id, def);
            self.defs[def].kind = Kind::Via(self.ty(module, &form, &[])?);
            return Ok(Ty::Named(def));
        }
        if let Item::Type(alias) = &item {
            let params: Vec<String> = alias
                .generics
                .type_params()
                .map(|p| p.ident.to_string())
                .collect();
            let subst: Vec<(String, Ty)> = params.into_iter().zip(args).collect();
            return self.ty(id.module, &alias.ty, &subst);
        }
        if let Item::Trait(_) = item {
            return Err(format!("trait {name} used as a type"));
        }
        if let Some(&def) = self.by_item.get(&id) {
            return Ok(Ty::Named(def));
        }
        let rust = self.public_path(id, name);
        let wit = self.unique_wit(id, name);
        let lifetimes = generics(&item).lifetimes().count();
        // Reserve the slot first: a type may refer to itself.
        let def = self.push(Def {
            rust,
            wit: wit.clone(),
            lifetimes,
            kind: Kind::Opaque("recursive".into()),
        });
        self.by_item.insert(id, def);
        let kind = self.kind(id, &item, name, &wit)?;
        self.defs[def].kind = kind;
        Ok(Ty::Named(def))
    }

    fn kind(&mut self, id: ItemId, item: &Item, name: &str, wit: &str) -> Result<Kind, String> {
        let serde = derives(item, "Serialize") && derives(item, "Deserialize");
        if generics(item).type_params().next().is_some() {
            return Ok(if serde {
                Kind::Json
            } else {
                Kind::Opaque(format!("{name} is generic"))
            });
        }
        let structural = match item {
            Item::Struct(item) => match &item.fields {
                Fields::Named(fields)
                    if fields.named.iter().all(|f| self.reachable(id, &f.vis)) =>
                {
                    self.fields(id.module, fields.named.iter())
                        .map(Kind::Record)
                }
                Fields::Unnamed(fields)
                    if fields.unnamed.iter().all(|f| self.reachable(id, &f.vis)) =>
                {
                    fields
                        .unnamed
                        .iter()
                        .map(|f| self.ty(id.module, &f.ty, &[]))
                        .collect::<Result<_, _>>()
                        .map(Kind::TupleRecord)
                }
                Fields::Unit => Ok(Kind::Unit),
                _ => Err(format!("{name} has private fields")),
            },
            Item::Enum(item)
                if item
                    .variants
                    .iter()
                    .all(|v| matches!(v.fields, Fields::Unit)) =>
            {
                Ok(Kind::Enum(
                    item.variants.iter().map(|v| v.ident.to_string()).collect(),
                ))
            }
            Item::Enum(item) => item
                .variants
                .iter()
                .map(|v| {
                    let payload = match &v.fields {
                        Fields::Unit => Payload::None,
                        Fields::Unnamed(fields) => Payload::Tuple(
                            fields
                                .unnamed
                                .iter()
                                .map(|f| self.ty(id.module, &f.ty, &[]))
                                .collect::<Result<_, _>>()?,
                        ),
                        Fields::Named(fields) => Payload::Record(
                            format!("{wit}-{}", v.ident.to_string().to_kebab_case()),
                            self.fields(id.module, fields.named.iter())?,
                        ),
                    };
                    Ok(Case {
                        rust: v.ident.to_string(),
                        wit: v.ident.to_string().to_kebab_case(),
                        payload,
                    })
                })
                .collect::<Result<_, String>>()
                .map(Kind::Variant),
            _ => Err(format!("{name} is not a struct or enum")),
        };
        Ok(match structural {
            Ok(kind) => kind,
            Err(_) if serde => Kind::Json,
            Err(why) => Kind::Opaque(why),
        })
    }

    fn fields<'f>(
        &mut self,
        module: usize,
        fields: impl Iterator<Item = &'f syn::Field>,
    ) -> Result<Vec<Field>, String> {
        fields
            .map(|f| {
                let rust = f.ident.as_ref().unwrap().to_string();
                Ok(Field {
                    wit: wit_ident(&rust),
                    ty: self.ty(module, &f.ty, &[])?,
                    rust,
                })
            })
            .collect()
    }

    /// The path generated glue uses for item `id`: `mod_api::Name` when `mod_api`'s root
    /// exports it, its defining module's path otherwise.
    fn public_path(&mut self, id: ItemId, name: &str) -> String {
        if let Ok(Found::Item(root)) = self.index.resolve(self.api_root, name)
            && root == id
        {
            return format!("mod_api::{name}");
        }
        let module = &self.index.modules[id.module];
        let mut path = vec![module.krate.to_string()];
        path.extend(module.path.iter().cloned());
        path.push(name.to_string());
        path.join("::")
    }

    /// A WIT name for `name` no other definition has.
    fn unique_wit(&self, id: ItemId, name: &str) -> String {
        let base = name.to_kebab_case();
        if !self.wit_names.contains_key(&base) {
            return base;
        }
        let module = &self.index.modules[id.module];
        let prefix = module.path.last().map_or(module.krate, String::as_str);
        format!("{}-{base}", prefix.to_kebab_case())
    }
}

fn primitive(name: &str) -> Option<Ty> {
    Some(match name {
        "bool" => Ty::Prim("bool", "bool"),
        "u8" => Ty::Prim("u8", "u8"),
        "u16" => Ty::Prim("u16", "u16"),
        "u32" => Ty::Prim("u32", "u32"),
        "u64" => Ty::Prim("u64", "u64"),
        "usize" => Ty::Prim("usize", "u64"),
        "i8" => Ty::Prim("i8", "s8"),
        "i16" => Ty::Prim("i16", "s16"),
        "i32" => Ty::Prim("i32", "s32"),
        "i64" => Ty::Prim("i64", "s64"),
        "isize" => Ty::Prim("isize", "s64"),
        "f32" => Ty::Prim("f32", "f32"),
        "f64" => Ty::Prim("f64", "f64"),
        "char" => Ty::Prim("char", "char"),
        "u128" => Ty::Wide("u128"),
        "i128" => Ty::Wide("i128"),
        _ => return None,
    })
}

fn trait_name(object: &syn::TypeTraitObject) -> Result<String, String> {
    object
        .bounds
        .iter()
        .find_map(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => {
                bound.path.segments.last().map(|s| s.ident.to_string())
            }
            _ => None,
        })
        .ok_or_else(|| format!("dyn {}", object.to_token_stream()))
}

fn generics(item: &Item) -> syn::Generics {
    match item {
        Item::Struct(item) => item.generics.clone(),
        Item::Enum(item) => item.generics.clone(),
        Item::Type(item) => item.generics.clone(),
        _ => syn::Generics::default(),
    }
}

fn derives(item: &Item, name: &str) -> bool {
    let attrs = match item {
        Item::Struct(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        _ => return false,
    };
    attrs.iter().any(|attr| {
        attr.path().is_ident("derive")
            && attr
                .to_token_stream()
                .to_string()
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|word| word == name)
    })
}

impl Types<'_> {
    /// Whether the generated glue, inside `mod_api`, can reach a field of item `id` with
    /// visibility `vis`: a public one, a crate-visible one of `mod_api`'s, or any of a type
    /// defined at `mod_api`'s root, which the glue is a descendant of.
    fn reachable(&self, id: ItemId, vis: &syn::Visibility) -> bool {
        let in_api = self.index.modules[id.module].krate == "mod_api";
        match vis {
            syn::Visibility::Public(_) => true,
            syn::Visibility::Restricted(r) => in_api && r.path.is_ident("crate"),
            syn::Visibility::Inherited => id.module == self.api_root,
        }
    }
}

/// An item's attributes.
fn attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Struct(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        _ => &[],
    }
}

/// The form `#[wasm(via = Form)]` names.
fn via(attrs: &[syn::Attribute]) -> Option<Type> {
    attrs.iter().find_map(|attr| {
        if attr.path().segments.last()?.ident != "wasm" {
            return None;
        }
        let mut form = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("via") {
                form = Some(meta.value()?.parse::<Type>()?);
            }
            Ok(())
        })
        .ok()?;
        form
    })
}

/// The name a `use` tree of one path brings in.
fn use_name(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => use_name(&path.tree),
        syn::UseTree::Name(name) => Some(name.ident.to_string()),
        syn::UseTree::Rename(rename) => Some(rename.rename.to_string()),
        _ => None,
    }
}

/// The WIT identifier for a Rust field or function name.
pub(super) fn wit_ident(rust: &str) -> String {
    let rust = rust.trim_start_matches("r#");
    let kebab = rust.to_kebab_case();
    // A WIT word can't start with a digit: join it to the word before.
    let mut out = String::new();
    for (i, word) in kebab.split('-').enumerate() {
        if i > 0 && !word.starts_with(|c: char| c.is_ascii_digit()) {
            out.push('-');
        }
        out.push_str(word);
    }
    if WIT_KEYWORDS.contains(&out.as_str()) {
        format!("%{out}")
    } else {
        out
    }
}

/// The Rust identifier both bindings generators give a WIT field or function.
pub(super) fn bound_ident(wit: &str) -> String {
    let name = wit.trim_start_matches('%').replace('-', "_");
    if RUST_KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// The Rust type or case name both bindings generators give a WIT name.
pub(super) fn bound_type(wit: &str) -> String {
    wit.trim_start_matches('%').to_upper_camel_case()
}

const WIT_KEYWORDS: &[&str] = &[
    "use",
    "type",
    "func",
    "u8",
    "u16",
    "u32",
    "u64",
    "s8",
    "s16",
    "s32",
    "s64",
    "f32",
    "f64",
    "float32",
    "float64",
    "char",
    "record",
    "resource",
    "own",
    "borrow",
    "flags",
    "variant",
    "enum",
    "bool",
    "string",
    "option",
    "result",
    "future",
    "stream",
    "error-context",
    "list",
    "_",
    "as",
    "from",
    "static",
    "interface",
    "tuple",
    "import",
    "export",
    "world",
    "package",
    "constructor",
    "include",
    "with",
    "async",
    "map",
];

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];
