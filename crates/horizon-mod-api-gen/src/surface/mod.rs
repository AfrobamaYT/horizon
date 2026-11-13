//! The surface export: `surface.json` ([`mod_surface::Surface`]) from what the build parses. Hooks
//! come from the catalog, with their arguments from the generated event types; trait roles,
//! sides and "Not on wasm" from the wire index; everything else from a walk of the public items
//! of `horizon-mod-api` and the vocabulary it re-exports.
mod rust;
#[cfg(test)]
mod tests;
mod walk;

use crate::catalog::{self, Catalog};
use crate::index::{Found, Index, ItemId};
use mod_surface::shape::Shapes;
use mod_surface::*;
use quote::ToTokens;
use rust::{Layouts, Paths, param_names, print};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use syn::visit_mut::VisitMut;
use syn::{Item, Visibility};
use walk::{attrs, compiled, hidden};

/// What the wire index says about the traits that cross the boundary.
#[derive(Default)]
pub(crate) struct Boundary {
    /// By trait name.
    roles: BTreeMap<String, Role>,
    sides: BTreeMap<String, Side>,
    /// (trait, method) → why it cannot cross.
    unbound: BTreeMap<(String, String), String>,
    /// An area trait → the union trait it is part of.
    unions: BTreeMap<String, String>,
}

/// A trait's name without its path or arguments (`ResourceHandler<ItemResource>` → `ResourceHandler`).
fn trait_name(owner: &str) -> String {
    let bare = owner.split('<').next().unwrap_or(owner);
    bare.rsplit("::").next().unwrap_or(bare).to_owned()
}

impl Boundary {
    fn from_wire(surface: &crate::wire::Surface<'_>) -> Self {
        let mut boundary = Self::default();
        for owner in crate::wire::imports() {
            boundary.roles.insert(trait_name(owner), Role::Call);
        }
        for owner in crate::wire::exports() {
            boundary.roles.insert(trait_name(owner), Role::Implement);
        }
        for (owner, area) in &surface.parts {
            boundary.unions.insert(trait_name(area), trait_name(owner));
        }
        for unbound in &surface.unbound {
            boundary.unbound.insert(
                (trait_name(unbound.owner), unbound.name.clone()),
                unbound.why.clone(),
            );
        }
        boundary.sides.insert("ServerHost".into(), Side::Server);
        boundary.sides.insert("ClientHost".into(), Side::Client);
        for (owner, accessor, _) in crate::wire::REACHED {
            let side = if accessor.starts_with("server_host") {
                Side::Server
            } else if accessor.starts_with("client_host") {
                Side::Client
            } else {
                continue;
            };
            boundary.sides.insert(trait_name(owner), side);
        }
        boundary
    }

    fn find<'b, T>(&'b self, map: &'b BTreeMap<String, T>, name: &str) -> Option<&'b T> {
        map.get(name)
            .or_else(|| self.unions.get(name).and_then(|union| map.get(union)))
    }

    fn role(&self, name: &str) -> Role {
        self.find(&self.roles, name).copied().unwrap_or(Role::None)
    }

    fn side(&self, name: &str) -> Side {
        self.find(&self.sides, name).copied().unwrap_or(Side::Both)
    }

    fn unbound(&self, name: &str, method: &str) -> Option<&String> {
        self.unbound
            .get(&(name.to_owned(), method.to_owned()))
            .or_else(|| {
                let union = self.unions.get(name)?;
                self.unbound.get(&(union.clone(), method.to_owned()))
            })
    }
}

/// The export, with what it could not classify.
pub struct Export {
    pub surface: Surface,
    /// One line per public item or declaration the export has no record kind for.
    pub notes: Vec<String>,
}

impl Export {
    /// How many records of each kind, members included.
    pub fn counts(&self) -> Vec<(&'static str, usize)> {
        let s = &self.surface;
        vec![
            ("hooks", s.hooks.len()),
            ("traits", s.traits.len()),
            ("methods", s.traits.iter().map(|t| t.methods.len()).sum()),
            ("handles", s.handles.len()),
            ("functions", s.functions.len()),
            ("macros", s.macros.len()),
            ("types", s.types.len()),
            ("fields", s.types.iter().map(|t| t.fields.len()).sum()),
            ("variants", s.types.iter().map(|t| t.variants.len()).sum()),
            ("parts", s.parts.len()),
            ("capabilities", s.capabilities.len()),
        ]
    }
}

/// Exports the surface of the source tree at `root`.
pub fn export(root: &Path) -> Result<Export, String> {
    // The wire index's resolver recurses through the API's types: give it a deep stack.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(scope, || export_here(root))
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "the surface export panicked".to_string())?
    })
}

fn export_here(root: &Path) -> Result<Export, String> {
    let catalog = Catalog::load(root).map_err(|e| e.to_string())?;
    let generated = crate::sources(root, crate::FillMode::Require)?;
    let included = [
        (crate::host::AREAS_OUTPUT, generated.areas.as_str()),
        ("mod_api_generated.rs", generated.hooks.as_str()),
        ("mod_api_generated_host.rs", generated.host.as_str()),
    ];
    let providers = crate::capability_providers::generate(root, &included)?.0;
    let mut with_providers = included.to_vec();
    with_providers.push(("capability_providers_generated.rs", providers.as_str()));
    let wire = crate::wire::surface(root, &with_providers)?;
    let boundary = Boundary::from_wire(&wire);
    let mut index = wire.types.index;
    build(root, &mut index, &catalog.hooks, &boundary)
}

/// The `///` text of `attrs`, one line each, a leading space dropped.
fn doc(attrs: &[syn::Attribute]) -> String {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) if pair.path.is_ident("doc") => match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    lines
        .iter()
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// A doc's `Not on wasm: <why>`, to the end of its paragraph.
fn not_on_wasm(attrs: &[syn::Attribute]) -> Option<String> {
    let text = doc(attrs);
    let mut lines = text.lines().map(str::trim);
    let first = lines.find_map(|line| line.strip_prefix("Not on wasm: "))?;
    let why: Vec<&str> = std::iter::once(first)
        .chain(lines.take_while(|line| !line.is_empty()))
        .collect();
    Some(why.join(" ").trim().to_owned()).filter(|why| why.len() > 1)
}

fn wasm(attrs: &[syn::Attribute]) -> Wasm {
    not_on_wasm(attrs).map_or(Wasm::Yes, Wasm::No)
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn derives(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("derive")) {
        if let Ok(paths) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
        ) {
            for path in paths {
                if let Some(last) = path.segments.last() {
                    out.insert(last.ident.to_string());
                }
            }
        }
    }
    out.into_iter().collect()
}

/// Normalises types and signatures written in one module.
struct Writer<'p, 'a> {
    index: &'p mut Index<'a>,
    public: &'p HashMap<ItemId, Vec<String>>,
}

impl<'a> Writer<'_, 'a> {
    fn paths(&mut self, module: usize, params: &[String]) -> Paths<'_, 'a> {
        Paths {
            index: self.index,
            public: self.public,
            module,
            params: params.to_vec(),
            refs: BTreeSet::new(),
        }
    }

    fn ty(&mut self, module: usize, params: &[String], ty: &syn::Type) -> (String, Vec<String>) {
        let mut ty = ty.clone();
        let mut paths = self.paths(module, params);
        paths.visit_type_mut(&mut ty);
        (
            print(ty.to_token_stream()),
            paths.refs.into_iter().collect(),
        )
    }

    fn signature(
        &mut self,
        module: usize,
        params: &[String],
        sig: &syn::Signature,
    ) -> (String, Vec<String>) {
        let mut sig = sig.clone();
        for input in &mut sig.inputs {
            match input {
                syn::FnArg::Receiver(receiver) => receiver.attrs.clear(),
                syn::FnArg::Typed(arg) => {
                    arg.attrs.clear();
                    // A parameter's name is not part of what a caller relies on.
                    *arg.pat = syn::parse_quote!(_);
                }
            }
        }
        let mut params = params.to_vec();
        params.extend(param_names(&sig.generics));
        let mut paths = self.paths(module, &params);
        paths.visit_signature_mut(&mut sig);
        (
            print(sig.to_token_stream()),
            paths.refs.into_iter().collect(),
        )
    }

    fn generics(&mut self, module: usize, generics: &syn::Generics) -> (String, Vec<String>) {
        let mut generics = generics.clone();
        let params = param_names(&generics);
        let mut paths = self.paths(module, &params);
        paths.visit_generics_mut(&mut generics);
        let mut text = print(generics.to_token_stream());
        if let Some(clause) = &generics.where_clause {
            text.push(' ');
            text.push_str(&print(clause.to_token_stream()));
        }
        (text.trim().to_owned(), paths.refs.into_iter().collect())
    }
}

fn merge(into: &mut Vec<String>, refs: Vec<String>) {
    into.extend(refs);
    into.sort();
    into.dedup();
}

/// A type before its shape is known.
struct Pending {
    id: String,
    item: ItemId,
    doc: String,
    shaped: bool,
    contract: TypeContract,
    fields: Vec<Field>,
    variants: Vec<Variant>,
}

/// The surface of the API whose modules `index` reads, with `hooks` from the catalog.
pub(crate) fn build(
    root: &Path,
    index: &mut Index<'_>,
    hooks: &[catalog::Hook],
    boundary: &Boundary,
) -> Result<Export, String> {
    let mut walker = walk::Walker::new(index);
    let public = walker.public()?;
    let mut notes = public.notes.clone();
    // Inherent methods live in any module of the crate that defines the type.
    let mut crates: BTreeSet<&'static str> = BTreeSet::from(["mod_api"]);
    for id in public.items.keys() {
        crates.insert(walker.index.modules[id.module].krate);
    }
    let mut modules = Vec::new();
    for krate in crates {
        modules.extend(walker.all_modules(krate)?);
    }
    notes.extend(walker.take_notes());
    // Every name a mod can write; what is behind `#[doc(hidden)]` is recorded once a recorded
    // signature names it.
    let mut nameable = public.items.clone();
    nameable.extend(public.hidden.iter().map(|(id, path)| (*id, path.clone())));
    let by_path: HashMap<String, ItemId> = nameable
        .iter()
        .map(|(id, path)| (path.join("::"), *id))
        .collect();
    let mut writer = Writer {
        index,
        public: &nameable,
    };
    let inherent = inherent_methods(&mut writer, &modules, &nameable);
    let mut queue: Vec<(ItemId, Vec<String>)> = public
        .items
        .iter()
        .map(|(id, path)| (*id, path.clone()))
        .collect();
    queue.sort_by(|a, b| b.1.cmp(&a.1));
    // A hook's event type is part of the surface even when no public signature names it (a
    // scoped hook's event).
    let events: BTreeSet<String> = hooks
        .iter()
        .map(|hook| format!("{}Event", crate::hook_id(&hook.name)))
        .collect();
    let mut hidden_events: Vec<(ItemId, Vec<String>)> = public
        .hidden
        .iter()
        .filter(|(_, path)| path.last().is_some_and(|name| events.contains(name)))
        .map(|(id, path)| (*id, path.clone()))
        .collect();
    hidden_events.sort_by(|a, b| b.1.cmp(&a.1));
    queue.extend(hidden_events);
    let mut recorded: std::collections::HashSet<ItemId> = queue.iter().map(|(id, _)| *id).collect();
    let mut layouts = Layouts::new();
    let mut traits = Vec::new();
    let mut functions = Vec::new();
    let mut pending = Vec::new();
    let mut capabilities = Vec::new();
    let mut handles: BTreeMap<String, Function> = BTreeMap::new();
    while let Some((id, path)) = queue.pop() {
        let path = path.join("::");
        let module = id.module;
        let item = writer.index.item(id).clone();
        let mut named: Vec<String> = Vec::new();
        match &item {
            Item::Struct(_) | Item::Enum(_) | Item::Union(_) | Item::Type(_) => {
                let record = type_record(&mut writer, id, &path, &item);
                named.extend(record.contract.refs.iter().cloned());
                for field in &record.fields {
                    named.extend(field.contract.refs.iter().cloned());
                }
                for variant in &record.variants {
                    named.extend(variant.contract.refs.iter().cloned());
                }
                pending.push(record);
                for (impl_module, params, method) in inherent.get(&id).into_iter().flatten() {
                    let handle_id = format!("handle:{path}::{}", method.sig.ident);
                    if handles.contains_key(&handle_id) {
                        notes.push(format!(
                            "{handle_id}: a second inherent method of that name (another impl block), not recorded"
                        ));
                        continue;
                    }
                    let (signature, refs) = writer.signature(*impl_module, params, &method.sig);
                    named.extend(refs.iter().cloned());
                    handles.insert(
                        handle_id.clone(),
                        Function::new(
                            handle_id,
                            doc(&method.attrs),
                            FunctionContract {
                                signature,
                                wasm: wasm(&method.attrs),
                                refs,
                            },
                        ),
                    );
                }
            }
            Item::Trait(item) => {
                let record = trait_record(&mut writer, boundary, module, &path, item);
                named.extend(record.contract.refs.iter().cloned());
                for method in &record.methods {
                    named.extend(method.contract.refs.iter().cloned());
                }
                traits.push(record);
            }
            Item::Fn(function) => {
                let (signature, refs) = writer.signature(module, &[], &function.sig);
                named.extend(refs.iter().cloned());
                functions.push(Function::new(
                    format!("fn:{path}"),
                    doc(&function.attrs),
                    FunctionContract {
                        signature,
                        wasm: wasm(&function.attrs),
                        refs,
                    },
                ));
            }
            Item::Const(constant) => match capability_key(&mut writer, module, &path, constant) {
                Some(capability) => {
                    named.extend(capability.contract.refs.iter().cloned());
                    capabilities.push(capability);
                }
                None => notes.push(format!("constant {path}: no record kind for constants")),
            },
            Item::Static(_) => notes.push(format!("static {path}: no record kind for statics")),
            _ => notes.push(format!("{path}: not a kind the surface records")),
        }
        for reference in named {
            let Some((_, target)) = reference.split_once(':') else {
                continue;
            };
            if let Some(next) = by_path.get(target)
                && recorded.insert(*next)
            {
                queue.push((*next, nameable[next].clone()));
            }
        }
    }

    let macros = macros(root, writer.index, &public.macros, &mut notes)?;

    // Shapes, over every type the public types contain.
    let keys: Vec<Option<String>> = pending
        .iter()
        .map(|record| {
            record
                .shaped
                .then(|| layouts.key(writer.index, record.item))
        })
        .collect();
    layouts.finish(writer.index);
    let mut shapes = Shapes::new(&layouts.table);
    let mut types = Vec::new();
    for (record, key) in pending.into_iter().zip(keys) {
        let shape = match key {
            Some(key) => shapes
                .shape(&key)
                .map_err(|e| format!("{}: {e}", record.id))?,
            None => String::new(),
        };
        types.push(Type::new(
            record.id,
            record.doc,
            shape,
            record.contract,
            record.fields,
            record.variants,
        ));
    }

    let hooks = hook_records(hooks, &types, &mut notes);
    let parts = part_records(&hooks, &traits, &mut notes);
    capabilities.extend(manifest_capabilities(root, &mut notes));
    notes.sort();
    notes.dedup();
    Ok(Export {
        surface: Surface::new(
            hooks,
            traits,
            handles.into_values().collect(),
            functions,
            macros,
            types,
            parts,
            capabilities,
        ),
        notes,
    })
}

/// The public inherent methods of every nameable type, by type: the impl's module, its
/// generic parameter names (and `Self`), and the method.
type Inherent = HashMap<ItemId, Vec<(usize, Vec<String>, syn::ImplItemFn)>>;

fn inherent_methods(
    writer: &mut Writer<'_, '_>,
    modules: &[usize],
    nameable: &HashMap<ItemId, Vec<String>>,
) -> Inherent {
    let mut out: Inherent = HashMap::new();
    for &module in modules {
        let module_items = writer.index.modules[module].items.clone();
        for item in module_items {
            let Item::Impl(block) = item else { continue };
            if block.trait_.is_some() || !compiled(&block.attrs) {
                continue;
            }
            let syn::Type::Path(self_ty) = &*block.self_ty else {
                continue;
            };
            let segments: Vec<String> = self_ty
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            let Ok(Found::Item(owner)) = writer.index.resolve_path(module, &segments) else {
                continue;
            };
            if !nameable.contains_key(&owner) {
                continue;
            }
            let mut params = param_names(&block.generics);
            params.push("Self".into());
            for impl_item in &block.items {
                let syn::ImplItem::Fn(method) = impl_item else {
                    continue;
                };
                if matches!(method.vis, Visibility::Public(_))
                    && compiled(&method.attrs)
                    && !hidden(&method.attrs)
                {
                    out.entry(owner)
                        .or_default()
                        .push((module, params.clone(), method.clone()));
                }
            }
        }
    }
    out
}

fn trait_record(
    writer: &mut Writer<'_, '_>,
    boundary: &Boundary,
    module: usize,
    path: &str,
    item: &syn::ItemTrait,
) -> Trait {
    let name = item.ident.to_string();
    let role = boundary.role(&name);
    let (generics, mut refs) = writer.generics(module, &item.generics);
    let mut params = param_names(&item.generics);
    params.push("Self".into());
    let mut bounds = Vec::new();
    for bound in &item.supertraits {
        let mut bound = bound.clone();
        let mut paths = writer.paths(module, &params);
        paths.visit_type_param_bound_mut(&mut bound);
        let found: Vec<String> = paths.refs.into_iter().collect();
        merge(&mut refs, found);
        bounds.push(print(bound.to_token_stream()));
    }
    let mut methods = Vec::new();
    let mut required = Vec::new();
    for member in &item.items {
        let syn::TraitItem::Fn(method) = member else {
            continue;
        };
        if hidden(&method.attrs) || !compiled(&method.attrs) {
            continue;
        }
        let method_name = method.sig.ident.to_string();
        let (signature, method_refs) = writer.signature(module, &params, &method.sig);
        let wasm = match not_on_wasm(&method.attrs) {
            Some(why) => Wasm::No(why),
            None => match boundary.unbound(&name, &method_name) {
                Some(why) if role != Role::None => Wasm::No(why.clone()),
                _ => Wasm::Yes,
            },
        };
        let is_required = method.default.is_none();
        if is_required && role == Role::Implement {
            required.push(Required {
                name: method_name.clone(),
                signature: signature.clone(),
            });
        }
        methods.push(Method::new(
            format!("method:{path}::{method_name}"),
            doc(&method.attrs),
            MethodContract {
                signature,
                required: is_required,
                wasm,
                refs: method_refs,
            },
        ));
    }
    Trait::new(
        format!("trait:{path}"),
        doc(&item.attrs),
        TraitContract {
            role,
            side: boundary.side(&name),
            generics,
            bounds,
            required,
            refs,
        },
        methods,
    )
}

fn type_record(writer: &mut Writer<'_, '_>, item_id: ItemId, path: &str, item: &Item) -> Pending {
    let module = item_id.module;
    let id = format!("type:{path}");
    let attrs = attrs(item).to_vec();
    let non_exhaustive = has_attr(&attrs, "non_exhaustive");
    let derives = derives(&attrs);
    let mut fields = Vec::new();
    let mut variants = Vec::new();
    let mut members = Vec::new();
    let mut alias_refs = Vec::new();
    let mut target = None;
    let empty = syn::Generics::default();
    let item_generics = match item {
        Item::Struct(s) => &s.generics,
        Item::Enum(e) => &e.generics,
        Item::Union(u) => &u.generics,
        Item::Type(t) => &t.generics,
        _ => &empty,
    };
    let (generics, _) = writer.generics(module, item_generics);
    let params = param_names(item_generics);
    let mut field_records =
        |writer: &mut Writer<'_, '_>, list: &syn::Fields| -> (bool, Vec<String>) {
            let mut all_public = true;
            let mut written = Vec::new();
            for (position, field) in list.iter().enumerate() {
                let (ty, refs) = writer.ty(module, &params, &field.ty);
                let name = field
                    .ident
                    .as_ref()
                    .map_or_else(|| position.to_string(), ToString::to_string);
                written.push(match &field.ident {
                    Some(ident) => format!("{ident}: {ty}"),
                    None => ty.clone(),
                });
                if !matches!(field.vis, Visibility::Public(_)) {
                    all_public = false;
                    continue;
                }
                if hidden(&field.attrs) {
                    continue;
                }
                fields.push(Field::new(
                    format!("{id}.{name}"),
                    doc(&field.attrs),
                    FieldContract { ty, refs },
                ));
            }
            (all_public, written)
        };
    let (kind, constructible, shaped) = match item {
        Item::Struct(s) => {
            let kind = match &s.fields {
                syn::Fields::Named(_) => TypeKind::Struct,
                syn::Fields::Unnamed(_) => TypeKind::TupleStruct,
                syn::Fields::Unit => TypeKind::UnitStruct,
            };
            let (all_public, written) = field_records(writer, &s.fields);
            let constructible = all_public && !non_exhaustive;
            if constructible {
                members = written;
            }
            (kind, constructible, true)
        }
        Item::Union(u) => {
            let (all_public, written) =
                field_records(writer, &syn::Fields::Named(u.fields.clone()));
            let constructible = all_public && !non_exhaustive;
            if constructible {
                members = written;
            }
            (TypeKind::Union, constructible, true)
        }
        Item::Enum(e) => {
            for variant in &e.variants {
                let mut arguments = Vec::new();
                let mut refs = Vec::new();
                for (position, field) in variant.fields.iter().enumerate() {
                    let (ty, found) = writer.ty(module, &params, &field.ty);
                    merge(&mut refs, found);
                    arguments.push(Argument {
                        name: field
                            .ident
                            .as_ref()
                            .map_or_else(|| position.to_string(), ToString::to_string),
                        ty,
                    });
                }
                let named = matches!(variant.fields, syn::Fields::Named(_));
                let name = variant.ident.to_string();
                members.push(match (&variant.fields, named) {
                    (syn::Fields::Unit, _) => name.clone(),
                    (_, true) => format!(
                        "{name} {{ {} }}",
                        arguments
                            .iter()
                            .map(|a| format!("{}: {}", a.name, a.ty))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    (_, false) => format!(
                        "{name}({})",
                        arguments
                            .iter()
                            .map(|a| a.ty.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
                if hidden(&variant.attrs) {
                    continue;
                }
                variants.push(Variant::new(
                    format!("{id}.{name}"),
                    doc(&variant.attrs),
                    VariantContract {
                        fields: arguments,
                        named,
                        refs,
                    },
                ));
            }
            if non_exhaustive {
                members.clear();
            }
            (TypeKind::Enum, !non_exhaustive, true)
        }
        Item::Type(alias) => {
            let (written, refs) = writer.ty(module, &params, &alias.ty);
            target = Some(written);
            alias_refs = refs;
            (TypeKind::Alias, false, false)
        }
        _ => unreachable!("only types reach here"),
    };
    Pending {
        id,
        item: item_id,
        doc: doc(&attrs),
        shaped,
        contract: TypeContract {
            kind,
            generics,
            wire: derives.iter().any(|d| d == "Wire"),
            derives,
            constructible,
            non_exhaustive,
            target,
            members,
            refs: alias_refs,
        },
        fields,
        variants,
    }
}

/// A capability key constant (`CapabilityKey<dyn T>`, `ItemCapabilityKey<…>`,
/// `EntityCapabilityKey<…, C>`), keyed by its `name`.
fn capability_key(
    writer: &mut Writer<'_, '_>,
    module: usize,
    path: &str,
    constant: &syn::ItemConst,
) -> Option<Capability> {
    let syn::Type::Path(ty) = &*constant.ty else {
        return None;
    };
    let last = ty.path.segments.last()?;
    let kind = match last.ident.to_string().as_str() {
        "CapabilityKey" => CapabilityKind::Block,
        "ItemCapabilityKey" => CapabilityKind::Item,
        "EntityCapabilityKey" => CapabilityKind::Entity,
        _ => return None,
    };
    let text = |expr: &syn::Expr| match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(key),
            ..
        }) => Some(key.value()),
        _ => None,
    };
    // A literal with a `name`, or a constructor given the name (`Key::new("horizon:x")`).
    let key = match &*constant.expr {
        syn::Expr::Struct(literal) => literal.fields.iter().find_map(|field| match &field.member {
            syn::Member::Named(name) if name == "name" => text(&field.expr),
            _ => None,
        }),
        syn::Expr::Call(call) => call.args.first().and_then(text),
        _ => None,
    }?;
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let mut types = args.args.iter().filter_map(|arg| match arg {
        syn::GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    let mut refs = Vec::new();
    let mut write = |writer: &mut Writer<'_, '_>, ty: Option<syn::Type>| {
        ty.map(|ty| {
            let (text, found) = writer.ty(module, &[], &ty);
            merge(&mut refs, found);
            text
        })
    };
    let handler = write(writer, types.next());
    let context = write(writer, types.next());
    Some(Capability::new(
        format!("capability:{key}"),
        doc(&constant.attrs),
        CapabilityContract {
            kind,
            constant: Some(path.to_owned()),
            handler,
            context,
            refs,
        },
    ))
}

/// The manifest's `capabilities` values: the variants of `horizon-mod-package`'s `Capability`.
fn manifest_capabilities(root: &Path, notes: &mut Vec<String>) -> Vec<Capability> {
    let path = root.join("crates/horizon-mod-package/src/manifest.rs");
    let parsed = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|source| syn::parse_file(&source).map_err(|e| e.to_string()));
    let file = match parsed {
        Ok(file) => file,
        Err(why) => {
            notes.push(format!("manifest capabilities: {why}"));
            return Vec::new();
        }
    };
    let Some(capability) = file.items.iter().find_map(|item| match item {
        Item::Enum(e) if e.ident == "Capability" => Some(e),
        _ => None,
    }) else {
        notes.push("manifest capabilities: no Capability enum in the manifest".into());
        return Vec::new();
    };
    capability
        .variants
        .iter()
        .map(|variant| {
            let key = heck::ToKebabCase::to_kebab_case(variant.ident.to_string().as_str());
            Capability::new(
                format!("capability:{key}"),
                doc(&variant.attrs),
                CapabilityContract {
                    kind: CapabilityKind::Manifest,
                    constant: None,
                    handler: None,
                    context: None,
                    refs: Vec::new(),
                },
            )
        })
        .collect()
}

/// The re-exported procedural macros, from their definitions, and the crate's exported
/// `macro_rules!` macros.
fn macros(
    root: &Path,
    index: &mut Index<'_>,
    reexported: &[(Vec<String>, String)],
    notes: &mut Vec<String>,
) -> Result<Vec<Macro>, String> {
    let mut out = Vec::new();
    if !reexported.is_empty() {
        let path = root.join("crates/horizon-mod-api-macros/src/lib.rs");
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file = syn::parse_file(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut defined: BTreeMap<String, (MacroKind, Vec<String>, String)> = BTreeMap::new();
        for item in &file.items {
            let Item::Fn(function) = item else { continue };
            for attr in &function.attrs {
                let entry = if attr.path().is_ident("proc_macro_attribute") {
                    (
                        function.sig.ident.to_string(),
                        MacroKind::Attribute,
                        Vec::new(),
                    )
                } else if attr.path().is_ident("proc_macro") {
                    (
                        function.sig.ident.to_string(),
                        MacroKind::Function,
                        Vec::new(),
                    )
                } else if attr.path().is_ident("proc_macro_derive") {
                    let Ok(args) = attr.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                    ) else {
                        continue;
                    };
                    let mut args = args.into_iter();
                    let Some(syn::Meta::Path(name)) = args.next() else {
                        continue;
                    };
                    let helpers = args
                        .filter_map(|meta| match meta {
                            syn::Meta::List(list) if list.path.is_ident("attributes") => list
                                .parse_args_with(
                                    syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
                                )
                                .ok(),
                            _ => None,
                        })
                        .flatten()
                        .map(|i| i.to_string())
                        .collect();
                    (print(name.to_token_stream()), MacroKind::Derive, helpers)
                } else {
                    continue;
                };
                defined.insert(entry.0, (entry.1, entry.2, doc(&function.attrs)));
            }
        }
        for (public, name) in reexported {
            match defined.get(name) {
                Some((kind, attributes, text)) => out.push(Macro::new(
                    format!("macro:{}", public.join("::")),
                    text.clone(),
                    MacroContract {
                        kind: *kind,
                        attributes: attributes.clone(),
                    },
                )),
                None => notes.push(format!(
                    "{}: re-exports {name}, which is no procedural macro of horizon-mod-api-macros",
                    public.join("::")
                )),
            }
        }
    }
    for krate in ["horizon_mod_api", "mod_api"] {
        let mut walker = walk::Walker::new(index);
        for module in walker.all_modules(krate)? {
            for item in &walker.index.modules[module].items {
                let Item::Macro(mac) = item else { continue };
                let Some(name) = &mac.ident else { continue };
                if !has_attr(&mac.attrs, "macro_export")
                    || hidden(&mac.attrs)
                    || !compiled(&mac.attrs)
                    || name.to_string().starts_with("__")
                {
                    continue;
                }
                out.push(Macro::new(
                    format!("macro:{name}"),
                    doc(&mac.attrs),
                    MacroContract {
                        kind: MacroKind::Declarative,
                        attributes: Vec::new(),
                    },
                ));
            }
        }
    }
    Ok(out)
}

fn hook_records(hooks: &[catalog::Hook], types: &[Type], notes: &mut Vec<String>) -> Vec<Hook> {
    let events: BTreeMap<&str, &Type> = types.iter().map(|t| (leaf(&t.id), t)).collect();
    let mut out = Vec::new();
    for hook in hooks {
        let event = format!("{}Event", crate::hook_id(&hook.name));
        let fields = match events.get(event.as_str()) {
            Some(event_type) => event_type.fields.as_slice(),
            None => {
                notes.push(format!(
                    "hook {}: its event {event} is not a mod_api type, so its arguments are not recorded",
                    hook.name
                ));
                &[]
            }
        };
        let output_name = hook
            .out
            .as_deref()
            .or((hook.contract == crate::Contract::Cancel).then_some("cancelled"));
        let mut arguments = Vec::new();
        let mut output = None;
        let mut refs = Vec::new();
        for field in fields {
            merge(&mut refs, field.contract.refs.clone());
            let argument = Argument {
                name: leaf(&field.id).to_owned(),
                ty: field.contract.ty.clone(),
            };
            if Some(argument.name.as_str()) == output_name {
                output = Some(argument);
            } else {
                arguments.push(argument);
            }
        }
        let snake = |kind: catalog::PartKind| match kind {
            catalog::PartKind::Block => "block",
            catalog::PartKind::BlockEntity => "block_entity",
            catalog::PartKind::Item => "item",
            catalog::PartKind::Menu => "menu",
            catalog::PartKind::Fluid => "fluid",
            catalog::PartKind::Entity => "entity",
        };
        out.push(Hook::new(
            format!("hook:{}", hook.name),
            hook.doc.clone(),
            HookEngine {
                target: hook.target.clone(),
                context: hook.context.map(|context| {
                    match context {
                        catalog::PhaseContext::Registration => "registration",
                        catalog::PhaseContext::DataLoad => "data_load",
                        catalog::PhaseContext::Loop => "loop",
                        catalog::PhaseContext::Worldgen => "worldgen",
                    }
                    .to_owned()
                }),
            },
            Counterparts {
                neoforge: hook.counterparts.neoforge.clone(),
                fabric: hook.counterparts.fabric.clone(),
                paper: hook.counterparts.paper.clone(),
            },
            HookContract {
                version: hook.version,
                side: match hook.side {
                    catalog::Side::Server => Side::Server,
                    catalog::Side::Client => Side::Client,
                    catalog::Side::Both => Side::Both,
                },
                contract: match hook.contract {
                    crate::Contract::Listen => HookKind::Listen,
                    crate::Contract::Cancel => HookKind::Cancel,
                    crate::Contract::Collect => HookKind::Collect,
                    crate::Contract::Replace => HookKind::Replace,
                },
                scope: hook.scope.as_ref().map(|scope| Scope {
                    kind: snake(scope.kind).to_owned(),
                    key: scope.key.clone(),
                }),
                cancellable: hook.cancellable,
                host: hook.host.map(|host| {
                    match host {
                        catalog::HostAccess::Read => "read",
                        catalog::HostAccess::Level => "level",
                    }
                    .to_owned()
                }),
                owner: hook.owner.clone(),
                answered_by_mod: hook.answered_by_mod,
                arguments,
                output,
                refs,
            },
        ));
    }
    out
}

/// One Part kind per kind the catalog's scoped hooks name, implemented through `<Kind>Part`.
fn part_records(hooks: &[Hook], traits: &[Trait], notes: &mut Vec<String>) -> Vec<Part> {
    let mut kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for hook in hooks {
        if let Some(scope) = &hook.contract.scope {
            kinds
                .entry(scope.kind.clone())
                .or_default()
                .push(hook.id.clone());
        }
    }
    let mut out = Vec::new();
    for (kind, scoped) in kinds {
        let name = format!(
            "{}Part",
            heck::ToUpperCamelCase::to_upper_camel_case(kind.as_str())
        );
        let Some(found) = traits.iter().find(|t| leaf(&t.id) == name) else {
            notes.push(format!("part {kind}: no public trait {name}"));
            continue;
        };
        out.push(Part::new(
            format!("part:{kind}"),
            found.doc.clone(),
            scoped,
            PartContract {
                trait_id: found.id.clone(),
            },
        ));
    }
    out
}
