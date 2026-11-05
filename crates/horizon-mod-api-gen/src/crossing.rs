//! How a game internal crosses to a wasm mod (#589 W): one rule for both sides, the mod's generated
//! view and the game's generated tables, read from the same `internals.json` signatures.
//!
//! - A **copy** is plain data all the way down, at most [`LEAVES`] scalars: it crosses as a value.
//! - A **handle** has fields or elements a mod walks into: it crosses as a node of a path. Its
//!   parts that are opaque are just not reachable.
//! - An **opaque** type crosses neither way: trait objects, closures, cells and locks, channels,
//!   GPU and window objects, raw pointers, and types this game doesn't list.
use crate::internals::Internal;
use std::collections::{BTreeMap, HashMap};
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

/// Scalars, strings and the like, each one leaf.
const SCALARS: &[&str] = &[
    "bool",
    "char",
    "f32",
    "f64",
    "i8",
    "i16",
    "i32",
    "i64",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "usize",
    "str",
    "String",
    "PathBuf",
    "Path",
    "Identifier",
    "ResourceLocation",
];

/// Wrappers that cross as what they hold.
const TRANSPARENT: &[&str] = &["Box", "Arc", "Rc", "Cow"];

/// Containers a mod walks by index or key: always handles.
const CONTAINERS: &[&str] = &[
    "Vec",
    "VecDeque",
    "HashMap",
    "BTreeMap",
    "HashSet",
    "BTreeSet",
    "IndexMap",
    "IndexSet",
    "FxHashMap",
    "FxHashSet",
    "AHashMap",
    "SmallVec",
];

/// Types that never cross.
const OPAQUE: &[&str] = &[
    "RefCell",
    "Cell",
    "OnceCell",
    "OnceLock",
    "LazyLock",
    "Mutex",
    "RwLock",
    "MutexGuard",
    "Sender",
    "Receiver",
    "SyncSender",
    "JoinHandle",
    "Thread",
    "Instant",
    "SystemTime",
    "File",
    "TcpStream",
    "UdpSocket",
    "PhantomData",
    "Weak",
    "AtomicBool",
    "AtomicU8",
    "AtomicU16",
    "AtomicU32",
    "AtomicU64",
    "AtomicUsize",
    "AtomicI32",
    "AtomicI64",
];

/// Crates whose types never cross: the GPU, the window, input devices, audio.
const OPAQUE_CRATES: &[&str] = &["wgpu", "winit", "gilrs", "cpal", "rodio", "glyphon"];

/// The game's named types, by path, as `internals.json` lists them: a struct's fields or an enum's
/// cases, each a type as written.
struct Named {
    /// Each field's or case field's type, in order, as written.
    fields: Vec<Type>,
    /// Generic: it has no one layout, so it is opaque.
    generic: bool,
}

/// The rule over one game's internals.
pub struct Rule {
    named: HashMap<String, Named>,
    /// Each type name's paths, for a name written without its module.
    by_name: HashMap<String, Vec<String>>,
    memo: std::cell::RefCell<HashMap<String, Crossing>>,
}

/// `a::b::C<'_>` as `a::b::C`.
fn bare(path: &str) -> String {
    path.split("::")
        .map(|segment| segment.split('<').next().unwrap_or(segment))
        .collect::<Vec<_>>()
        .join("::")
}

/// A struct's or enum's field types, from its signature.
fn fields_of(kind: &str, signature: &str) -> Option<Named> {
    let item: syn::Item = syn::parse_str(signature).ok()?;
    let (generics, fields): (&syn::Generics, Vec<Type>) = match (kind, &item) {
        ("struct", syn::Item::Struct(item)) => (
            &item.generics,
            item.fields.iter().map(|f| f.ty.clone()).collect(),
        ),
        ("enum", syn::Item::Enum(item)) => (
            &item.generics,
            item.variants
                .iter()
                .flat_map(|v| v.fields.iter().map(|f| f.ty.clone()))
                .collect(),
        ),
        _ => return None,
    };
    let generic = generics
        .params
        .iter()
        .any(|param| !matches!(param, syn::GenericParam::Lifetime(_)));
    Some(Named { fields, generic })
}

impl Rule {
    /// The rule over `internals`: every struct and enum they list.
    pub fn new(internals: &[Internal]) -> Self {
        let mut named = HashMap::new();
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        for internal in internals {
            // One layout per path: a type guarded by a `cfg` is the build's own alternative.
            if !internal.cfg.is_empty() {
                continue;
            }
            if let Some(fields) = fields_of(internal.kind, &internal.signature) {
                let path = bare(&internal.path);
                let name = path.rsplit("::").next().unwrap_or(&path).to_owned();
                by_name.entry(name).or_default().push(path.clone());
                named.insert(path, fields);
            }
        }
        Self {
            named,
            by_name,
            memo: Default::default(),
        }
    }

    /// The listed type `ty` names, written in module `module`: its path as written if listed, else
    /// the one of its name in that module or above it, else the only one of its name.
    fn resolve(&self, module: &str, written: &syn::Path) -> Option<String> {
        let spelled = written
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if self.named.contains_key(&spelled) {
            return Some(spelled);
        }
        let name = written.segments.last()?.ident.to_string();
        let candidates = self.by_name.get(&name)?;
        let mut scope = module.to_owned();
        loop {
            let here = format!("{scope}::{spelled}");
            if self.named.contains_key(&here) {
                return Some(here);
            }
            match scope.rsplit_once("::") {
                Some((up, _)) => scope = up.to_owned(),
                None => break,
            }
        }
        (candidates.len() == 1).then(|| candidates[0].clone())
    }

    /// How `ty`, written in module `module`, crosses.
    pub fn crossing(&self, module: &str, ty: &Type) -> Crossing {
        self.of(module, ty, &mut Vec::new())
    }

    fn of(&self, module: &str, ty: &Type, open: &mut Vec<String>) -> Crossing {
        match ty {
            Type::Reference(r) => self.of(module, &r.elem, open),
            Type::Paren(p) => self.of(module, &p.elem, open),
            Type::Group(g) => self.of(module, &g.elem, open),
            Type::Tuple(t) => self.all(module, t.elems.iter(), open, 1),
            Type::Array(a) => {
                let n = match &a.len {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(n),
                        ..
                    }) => n.base10_parse::<usize>().ok(),
                    _ => None,
                };
                match (self.of(module, &a.elem, open), n) {
                    (Crossing::Copy(leaves), Some(n)) if leaves * n <= LEAVES => {
                        Crossing::Copy(leaves * n)
                    }
                    (Crossing::Opaque, _) => Crossing::Opaque,
                    _ => Crossing::Handle,
                }
            }
            Type::Slice(s) => match self.of(module, &s.elem, open) {
                Crossing::Opaque => Crossing::Opaque,
                _ => Crossing::Handle,
            },
            Type::Path(p) if p.qself.is_none() => self.path(module, &p.path, open),
            _ => Crossing::Opaque,
        }
    }

    fn path(&self, module: &str, path: &syn::Path, open: &mut Vec<String>) -> Crossing {
        let Some(last) = path.segments.last() else {
            return Crossing::Opaque;
        };
        let name = last.ident.to_string();
        let args: Vec<&Type> = match &last.arguments {
            PathArguments::AngleBracketed(a) => a
                .args
                .iter()
                .filter_map(|a| match a {
                    GenericArgument::Type(t) => Some(t),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        if path
            .segments
            .first()
            .is_some_and(|s| OPAQUE_CRATES.contains(&s.ident.to_string().as_str()))
            || OPAQUE.contains(&name.as_str())
        {
            return Crossing::Opaque;
        }
        if SCALARS.contains(&name.as_str()) && args.is_empty() {
            return Crossing::Copy(1);
        }
        if TRANSPARENT.contains(&name.as_str()) {
            return args
                .first()
                .map_or(Crossing::Opaque, |inner| self.of(module, inner, open));
        }
        if name == "Option" {
            return match args.first().map(|inner| self.of(module, inner, open)) {
                Some(Crossing::Copy(leaves)) if leaves < LEAVES => Crossing::Copy(leaves + 1),
                Some(Crossing::Copy(_)) | Some(Crossing::Handle) => Crossing::Handle,
                _ => Crossing::Opaque,
            };
        }
        if CONTAINERS.contains(&name.as_str()) {
            return if args
                .iter()
                .any(|a| self.of(module, a, open) == Crossing::Opaque)
            {
                Crossing::Opaque
            } else {
                Crossing::Handle
            };
        }
        let Some(found) = self.resolve(module, path) else {
            return Crossing::Opaque;
        };
        self.named_type(&found, open)
    }

    /// How the listed type at `path` crosses, each once.
    fn named_type(&self, path: &str, open: &mut Vec<String>) -> Crossing {
        if let Some(known) = self.memo.borrow().get(path) {
            return *known;
        }
        // A type that holds itself, through a box or a list, is walked, not copied.
        if open.iter().any(|p| p == path) {
            return Crossing::Handle;
        }
        let named = &self.named[path];
        let crossing = if named.generic {
            Crossing::Opaque
        } else {
            open.push(path.to_owned());
            let module = path.rsplit_once("::").map_or("", |(m, _)| m);
            let crossing = self.all(module, named.fields.iter(), open, 0);
            open.pop();
            crossing
        };
        self.memo.borrow_mut().insert(path.to_owned(), crossing);
        crossing
    }

    /// A type made of `parts`: a copy when each is and their leaves fit, else a handle, whose parts
    /// that can't cross simply can't be reached. A type with no parts is one leaf (`()`, a unit
    /// case).
    fn all<'t>(
        &self,
        module: &str,
        parts: impl Iterator<Item = &'t Type>,
        open: &mut Vec<String>,
        empty: usize,
    ) -> Crossing {
        let mut leaves = 0;
        let mut copy = true;
        let mut any = false;
        for part in parts {
            any = true;
            match self.of(module, part, open) {
                Crossing::Copy(n) => leaves += n,
                Crossing::Handle | Crossing::Opaque => copy = false,
            }
        }
        match (any, copy) {
            (false, _) => Crossing::Copy(empty.max(1)),
            (true, true) if leaves <= LEAVES => Crossing::Copy(leaves),
            _ => Crossing::Handle,
        }
    }

    /// How each listed type crosses, by path: what a report or a coverage test counts.
    pub fn types(&self) -> BTreeMap<&str, Crossing> {
        self.named
            .keys()
            .map(|path| (path.as_str(), self.named_type(path, &mut Vec::new())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal(path: &str, kind: &'static str, signature: &str) -> Internal {
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
