//! The public items of `mod_api`, each under the path a mod writes: the facade crate's root,
//! through public modules and `pub use`s (globs included), skipping `#[doc(hidden)]`, `__`
//! names and items compiled out of a mod's default native build.
use crate::index::{Found, Index, ItemId, uses};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use syn::{Item, Visibility};

/// The facade crate a mod depends on, and the crate its root re-exports.
const FACADE: &str = "horizon_mod_api";
const VOCAB: &str = "mod_api";
/// The vocabulary crate's own name, as the facade writes it.
const VOCAB_WRITTEN: &str = "mod_vocab";
/// The procedural macros crate whose macros the API re-exports.
pub(super) const MACROS: &str = "horizon_mod_api_macros";

/// What a public name leads to.
#[derive(Clone, Debug)]
pub(super) enum Target {
    Item(ItemId),
    Module(usize),
    External(Vec<String>),
}

/// A public name of a module.
#[derive(Clone)]
struct Name {
    name: String,
    target: Target,
    hidden: bool,
}

/// The walk's result.
#[derive(Default)]
pub(super) struct Public {
    /// Each public item with the path a mod writes, below `mod_api`.
    pub(super) items: HashMap<ItemId, Vec<String>>,
    /// Items a mod can name only through a `#[doc(hidden)]` item or route: part of the surface
    /// only when a public signature names them.
    pub(super) hidden: HashMap<ItemId, Vec<String>>,
    /// Re-exported procedural macros: the public path and the macro's name in its crate.
    pub(super) macros: Vec<(Vec<String>, String)>,
    /// What the walk could not classify, one line each.
    pub(super) notes: Vec<String>,
}

/// Whether `attrs` hold `#[doc(hidden)]`.
pub(super) fn hidden(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.meta, syn::Meta::List(list)
            if list.path.is_ident("doc") && list.tokens.to_string() == "hidden")
    })
}

/// Whether the items under `attrs` are part of a mod's default native build: `test` and every
/// `feature` are off, `target_arch` is not `wasm32`, anything else is on.
pub(super) fn compiled(attrs: &[syn::Attribute]) -> bool {
    fn holds(meta: &syn::Meta) -> bool {
        match meta {
            syn::Meta::Path(path) => !path.is_ident("test"),
            syn::Meta::NameValue(pair) => {
                let value = match &pair.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(text),
                        ..
                    }) => text.value(),
                    _ => String::new(),
                };
                if pair.path.is_ident("feature") {
                    false
                } else if pair.path.is_ident("target_arch") {
                    value != "wasm32"
                } else {
                    true
                }
            }
            syn::Meta::List(list) => {
                let Ok(inner) = list.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                ) else {
                    return true;
                };
                if list.path.is_ident("any") {
                    inner.iter().any(holds)
                } else if list.path.is_ident("all") {
                    inner.iter().all(holds)
                } else if list.path.is_ident("not") {
                    !inner.iter().all(holds)
                } else {
                    true
                }
            }
        }
    }
    attrs.iter().all(|attr| match &attr.meta {
        syn::Meta::List(list) if list.path.is_ident("cfg") => list
            .parse_args::<syn::Meta>()
            .map(|meta| holds(&meta))
            .unwrap_or(true),
        _ => true,
    })
}

pub(super) fn attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        _ => &[],
    }
}

fn defined(item: &Item) -> Option<(&Visibility, String)> {
    match item {
        Item::Const(i) => Some((&i.vis, i.ident.to_string())),
        Item::Enum(i) => Some((&i.vis, i.ident.to_string())),
        Item::Fn(i) => Some((&i.vis, i.sig.ident.to_string())),
        Item::Static(i) => Some((&i.vis, i.ident.to_string())),
        Item::Struct(i) => Some((&i.vis, i.ident.to_string())),
        Item::Trait(i) => Some((&i.vis, i.ident.to_string())),
        Item::Type(i) => Some((&i.vis, i.ident.to_string())),
        Item::Union(i) => Some((&i.vis, i.ident.to_string())),
        _ => None,
    }
}

pub(super) struct Walker<'i, 'a> {
    pub(super) index: &'i mut Index<'a>,
    names: HashMap<usize, Vec<Name>>,
    open: HashSet<usize>,
    notes: Vec<String>,
}

impl<'i, 'a> Walker<'i, 'a> {
    pub(super) fn new(index: &'i mut Index<'a>) -> Self {
        Self {
            index,
            names: HashMap::new(),
            open: HashSet::new(),
            notes: Vec::new(),
        }
    }

    fn module_name(&self, module: usize) -> String {
        let m = &self.index.modules[module];
        std::iter::once(m.krate)
            .chain(m.path.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("::")
    }

    /// The module a path written in `module` names, if it is one of an indexed crate.
    fn module_at(&mut self, module: usize, path: &[String]) -> Option<usize> {
        let mut path = path.to_vec();
        if path.first().map(String::as_str) == Some(VOCAB_WRITTEN) {
            path[0] = VOCAB.to_owned();
        }
        let (krate, inner) = self.index.absolute(module, &path)?;
        self.index.module(krate, &inner).ok()
    }

    /// What `path`, written in a `pub use` of `module`, leads to.
    fn target(&mut self, module: usize, path: &[String]) -> Result<Target, String> {
        if let Some(target) = self.module_at(module, path) {
            return Ok(Target::Module(target));
        }
        let (last, parent) = path.split_last().ok_or("an empty path")?;
        if parent.is_empty() {
            // A name the module does not define or import is an outside crate (`serde_json`).
            return Ok(match self.index.resolve(module, last) {
                Ok(Found::Item(id)) => Target::Item(id),
                Ok(Found::External(path)) => Target::External(path),
                Err(_) => Target::External(path.to_vec()),
            });
        }
        match self.module_at(module, parent) {
            Some(parent) => self
                .names(parent)?
                .into_iter()
                .find(|name| &name.name == last)
                .map(|name| name.target)
                .ok_or_else(|| {
                    format!(
                        "{last} is no public item of {} (a macro may make it)",
                        self.module_name(parent)
                    )
                }),
            None => Ok(Target::External(path.to_vec())),
        }
    }

    /// The public names of `module`: its public items and `pub use`s, then its globs' names.
    fn names(&mut self, module: usize) -> Result<Vec<Name>, String> {
        if let Some(names) = self.names.get(&module) {
            return Ok(names.clone());
        }
        if !self.open.insert(module) {
            return Ok(Vec::new());
        }
        let items = self.index.modules[module].items.clone();
        let mut own: Vec<Name> = Vec::new();
        let mut globbed: Vec<Name> = Vec::new();
        for (position, item) in items.iter().enumerate() {
            let attrs = attrs(item);
            if !compiled(attrs) {
                continue;
            }
            let hidden = hidden(attrs);
            match item {
                Item::Use(item) if matches!(item.vis, Visibility::Public(_)) => {
                    let mut found = Vec::new();
                    uses(&item.tree, &mut Vec::new(), &mut |path, alias| {
                        found.push((path, alias))
                    });
                    for (path, alias) in found {
                        match alias {
                            Some(alias) if alias.starts_with("__") => {}
                            Some(alias) => match self.target(module, &path) {
                                Ok(target) => own.push(Name {
                                    name: alias,
                                    target,
                                    hidden,
                                }),
                                Err(_) if hidden => {}
                                Err(why) => self.notes.push(format!(
                                    "pub use {} in {}: {why}",
                                    path.join("::"),
                                    self.module_name(module)
                                )),
                            },
                            None => match self.module_at(module, &path) {
                                Some(target) => {
                                    for mut name in self.names(target)? {
                                        name.hidden |= hidden;
                                        globbed.push(name);
                                    }
                                }
                                None => self.notes.push(format!(
                                    "pub use {}::* in {}: not a module of the API's crates",
                                    path.join("::"),
                                    self.module_name(module)
                                )),
                            },
                        }
                    }
                }
                Item::Mod(child) if matches!(child.vis, Visibility::Public(_)) => {
                    let krate = self.index.modules[module].krate;
                    let mut path = self.index.modules[module].path.clone();
                    path.push(child.ident.to_string());
                    let target = self.index.module(krate, &path)?;
                    own.push(Name {
                        name: child.ident.to_string(),
                        target: Target::Module(target),
                        hidden,
                    });
                }
                item => {
                    if let Some((vis, name)) = defined(item)
                        && matches!(vis, Visibility::Public(_))
                    {
                        own.push(Name {
                            name,
                            target: Target::Item(ItemId {
                                module,
                                item: position,
                            }),
                            hidden,
                        });
                    }
                }
            }
        }
        for name in globbed {
            if !own.iter().any(|own| own.name == name.name) {
                own.push(name);
            }
        }
        self.open.remove(&module);
        self.names.insert(module, own.clone());
        Ok(own)
    }

    /// Every public item from the facade's root, breadth first, each under its shortest path
    /// (the first in byte order among equals). Names starting `__` are never followed.
    pub(super) fn public(&mut self) -> Result<Public, String> {
        let root = self.index.module(FACADE, &[])?;
        let mut public = Public::default();
        let mut queue = VecDeque::from([(root, Vec::<String>::new(), false)]);
        let mut seen = HashSet::from([(root, false)]);
        let mut macros: BTreeMap<Vec<String>, String> = BTreeMap::new();
        while let Some((module, prefix, behind)) = queue.pop_front() {
            let mut names = self.names(module)?;
            names.sort_by(|a, b| a.name.cmp(&b.name));
            for name in names {
                if name.name.starts_with("__") {
                    continue;
                }
                let behind = behind || name.hidden;
                let mut path = prefix.clone();
                path.push(name.name.clone());
                match name.target {
                    Target::Module(target) => {
                        if seen.insert((target, behind)) {
                            queue.push_back((target, path, behind));
                        }
                    }
                    Target::Item(id) => {
                        let behind = behind || hidden(attrs(self.index.item(id)));
                        let map = if behind {
                            &mut public.hidden
                        } else {
                            &mut public.items
                        };
                        let better = |old: &Vec<String>| (path.len(), &path) < (old.len(), old);
                        match map.get(&id) {
                            Some(old) if !better(old) => {}
                            _ => {
                                map.insert(id, path);
                            }
                        }
                    }
                    Target::External(_) if behind => {}
                    Target::External(external) => {
                        if external.first().map(String::as_str) == Some(MACROS)
                            && external.len() == 2
                        {
                            macros.entry(path).or_insert_with(|| external[1].clone());
                        } else {
                            public.notes.push(format!(
                                "{} re-exports {}, outside the API's crates",
                                path.join("::"),
                                external.join("::")
                            ));
                        }
                    }
                }
            }
        }
        let shown = &public.items;
        public.hidden.retain(|id, _| !shown.contains_key(id));
        public.macros = macros.into_iter().collect();
        public.notes.append(&mut self.notes);
        Ok(public)
    }

    /// Every module of `krate` a default native build compiles, private ones included.
    pub(super) fn all_modules(&mut self, krate: &str) -> Result<Vec<usize>, String> {
        let root = self.index.module(krate, &[])?;
        let mut out = vec![root];
        let mut next = 0;
        while next < out.len() {
            let module = out[next];
            next += 1;
            let children: Vec<String> = self.index.modules[module]
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Mod(child) if compiled(&child.attrs) => Some(child.ident.to_string()),
                    _ => None,
                })
                .collect();
            for child in children {
                let mut path = self.index.modules[module].path.clone();
                path.push(child);
                match self.index.module(krate, &path) {
                    Ok(id) => out.push(id),
                    Err(why) => self.notes.push(format!("module {krate}::{}: {why}", path.join("::"))),
                }
            }
        }
        Ok(out)
    }

    pub(super) fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }
}
