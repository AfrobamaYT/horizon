//! The WebAssembly interface of the mod API: every method of the traits a mod calls (the hosts,
//! the backends, the drawing contexts) and every method it implements (`Hooks`, the parts, the
//! capability handlers), as one WIT world and the glue on both sides of it.
mod glue;
mod index;
mod sides;
mod ty;
mod wit;

use index::{Found, Index};
use std::path::Path;
use syn::{FnArg, Item, Pat, ReturnType, TraitItem};
use ty::{Ty, Types};

/// Traits a mod calls: their methods are the world's imports.
pub(super) const IMPORTS: &[&str] = &[
    "Host",
    "ServerHost",
    "ClientHost",
    "Backend",
    "BackendMut",
    "DrawContext",
    "BlockEntityDrawContext",
    "LevelHost",
    "LevelRandomHost",
    "LevelTicksHost",
    "LevelWriteHost",
    "ConfigFiles",
    "menu::SlotView",
    "Registrar",
];

/// Imported traits a call never hands over, each reached through another's method instead.
pub(super) const REACHED: &[(&str, &str, &str)] = &[
    ("ConfigFiles", "host", "Ok(h.config_files())"),
    (
        "Backend",
        "host",
        "h.backend().map(crate::BackendRef::open).ok_or_else(|| missing(\"game backend\"))",
    ),
    (
        "BackendMut",
        "server_host_mut",
        "crate::backend_mut_of(h).ok_or_else(|| missing(\"game backend\"))",
    ),
];

/// Traits a mod implements: their methods are the world's exports. The handler traits the
/// capability keys name join them (`exports`).
const EXPORTS: &[&str] = &[
    "Hooks",
    "BlockPart",
    "BlockEntityPart",
    "ItemPart",
    "MenuPart",
    "ServerReloadListener",
    "ClientReloadListener",
    "PlaceProvider",
];

/// Every exported trait: [`EXPORTS`], then the handler traits `mod_api`'s capability keys name
/// (`ResourceHandler<ItemResource>`), each instance of a generic one its own export. Set once,
/// by `surface`.
pub(super) fn exports() -> &'static [&'static str] {
    EXPORTED.get().map_or(EXPORTS, Vec::as_slice)
}

static EXPORTED: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();

/// The handler traits `mod_api`'s capability keys (`CapabilityKey<dyn T>` and the item and
/// entity forms) name, as written, in the order their keys are exported.
fn key_handlers(types: &mut Types<'_>) -> Result<Vec<String>, String> {
    let api = types.index.module("mod_api", &[])?;
    let names: Vec<String> = types.index.modules[api]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Use(item) if matches!(item.vis, syn::Visibility::Public(_)) => Some(&item.tree),
            _ => None,
        })
        .flat_map(use_names)
        .collect();
    let mut handlers = Vec::new();
    for name in names {
        let Ok(Found::Item(id)) = types.index.resolve(api, &name) else {
            continue;
        };
        let Item::Const(item) = types.index.item(id) else {
            continue;
        };
        let syn::Type::Path(path) = &*item.ty else {
            continue;
        };
        let Some(last) = path.path.segments.last() else {
            continue;
        };
        if !last.ident.to_string().ends_with("CapabilityKey") {
            continue;
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            continue;
        };
        if let Some(syn::GenericArgument::Type(syn::Type::TraitObject(object))) = args.args.first()
        {
            let handler = ty::trait_name(object)?;
            if !handlers.contains(&handler) {
                handlers.push(handler);
            }
        }
    }
    Ok(handlers)
}

/// Every name a `use` tree brings in.
fn use_names(tree: &syn::UseTree) -> Vec<String> {
    match tree {
        syn::UseTree::Path(path) => use_names(&path.tree),
        syn::UseTree::Name(name) => vec![name.ident.to_string()],
        syn::UseTree::Rename(rename) => vec![rename.rename.to_string()],
        syn::UseTree::Group(group) => group.items.iter().flat_map(use_names).collect(),
        syn::UseTree::Glob(_) => Vec::new(),
    }
}

/// An owner as written (`ResourceHandler<ItemResource>`): the trait's path, and its type arguments.
fn owner_parts(owner: &str) -> Result<(Vec<String>, Vec<syn::Type>), String> {
    let path: syn::Path = syn::parse_str(owner).map_err(|e| format!("{owner}: {e}"))?;
    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let args = match &path.segments.last().unwrap().arguments {
        syn::PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    Ok((segments, args))
}

/// The exported traits the mod itself answers, with no handler index, and the guest's
/// expression for its implementation of each; every other export is a registered handler's.
pub(super) const MOD_EXPORTS: &[(&str, &str)] = &[("Hooks", "super::hooks()")];

/// The guest's receiver of `owner`'s exports, if the mod itself answers them.
pub(super) fn mod_receiver(owner: &str) -> Option<&'static str> {
    MOD_EXPORTS
        .iter()
        .find(|(o, _)| *o == owner)
        .map(|(_, r)| *r)
}

/// The exported traits that list the hooks they subscribe to, and the method listing them.
pub(super) const SUBSCRIBED: &[(&str, &str)] = &[
    ("Hooks", "subscribed"),
    ("BlockPart", "block_subscribed"),
    ("BlockEntityPart", "block_entity_subscribed"),
    ("ItemPart", "item_subscribed"),
    ("MenuPart", "menu_subscribed"),
];

/// One trait method as it crosses.
#[derive(Debug)]
pub(super) struct Method {
    pub(super) owner: &'static str,
    pub(super) name: String,
    /// `Some(mutable)` for a `self` receiver.
    pub(super) receiver: Option<bool>,
    pub(super) params: Vec<(String, Ty)>,
    pub(super) ret: Ty,
}

/// A method that can't cross, and why.
#[derive(Debug)]
pub(super) struct Unbound {
    pub(super) owner: &'static str,
    pub(super) name: String,
    pub(super) why: String,
}

pub(super) struct Surface<'a> {
    pub(super) types: Types<'a>,
    pub(super) methods: Vec<Method>,
    pub(super) unbound: Vec<Unbound>,
}

/// The methods of the bound traits, with their types resolved. `hooks` and `host` are the
/// generated sources `mod_api` includes.
pub(super) fn surface<'a>(
    root: &'a Path,
    included: &'a [(&'a str, &'a str)],
) -> Result<Surface<'a>, String> {
    let mut types = Types::new(Index::new(root, included))?;
    let api = types.index.module("mod_api", &[])?;
    let mut exported: Vec<&'static str> = EXPORTS.to_vec();
    for handler in key_handlers(&mut types)? {
        if !exported.iter().any(|e| *e == handler) {
            exported.push(Box::leak(handler.into_boxed_str()));
        }
    }
    let _ = EXPORTED.set(exported);
    let mut methods = Vec::new();
    let mut unbound = Vec::new();
    for owner in IMPORTS.iter().chain(exports()) {
        let (path, args) = owner_parts(owner)?;
        let Found::Item(id) = types.index.resolve_path(api, &path)? else {
            return Err(format!("{owner} is not a mod_api trait"));
        };
        let Item::Trait(item) = types.index.item(id).clone() else {
            return Err(format!("{owner} is not a trait"));
        };
        // A generic trait's instance: its parameters as the owner's arguments.
        let mut subst = Vec::new();
        for (param, arg) in item.generics.type_params().zip(&args) {
            subst.push((param.ident.to_string(), types.ty(api, arg, &[])?));
        }
        for item in &item.items {
            let TraitItem::Fn(function) = item else {
                continue;
            };
            let name = function.sig.ident.to_string();
            if hidden(&function.attrs) || name.ends_with("subscribed") {
                continue;
            }
            match method(&mut types, id.module, owner, function, &subst) {
                Ok(method) => methods.push(method),
                Err(why) => unbound.push(Unbound { owner, name, why }),
            }
        }
    }
    wit::refuse(&mut types);
    // A method whose types can't all cross is unbound, with the first reason.
    let (bound, refused): (Vec<_>, Vec<_>) = methods
        .into_iter()
        .map(|method| match wit::signature(&types, &method) {
            // A borrowed view handed back has nothing on the mod's side to point into.
            Ok(_)
                if IMPORTS.contains(&method.owner)
                    && glue::borrowed_record(&types, &method.ret) =>
            {
                Err(Unbound {
                    owner: method.owner,
                    name: method.name,
                    why: "returns a borrowed view, which crosses only as a resource".into(),
                })
            }
            Ok(_) => Ok(method),
            Err(why) => Err(Unbound {
                owner: method.owner,
                name: method.name,
                why,
            }),
        })
        .partition(Result::is_ok);
    let methods = bound.into_iter().map(Result::unwrap).collect();
    unbound.extend(refused.into_iter().map(|r| r.err().unwrap()));
    Ok(Surface {
        types,
        methods,
        unbound,
    })
}

/// The wasm side of the mod API: the WIT package, and the Rust each side compiles.
pub struct Wasm {
    pub wit: String,
    pub guest: String,
    pub host: String,
    /// `Owner::method: why` for each method that doesn't cross.
    pub unbound: Vec<String>,
    /// The source files read.
    pub sources: Vec<std::path::PathBuf>,
}

/// Generates the wasm side from the mod API's sources and its generated files (`included`).
pub fn generate(root: &Path, included: &[(&str, &str)]) -> Result<Wasm, String> {
    let mut surface = surface(root, included)?;
    let hooks = hook_ids(&mut surface.types)?;
    let servers = servers(&mut surface.types)?;
    surface.types.views = views(&mut surface.types)?;
    let methods: Vec<&Method> = surface.methods.iter().collect();
    let wit = wit::package(&surface.types, &methods, &api_version(root)?)?;
    Ok(Wasm {
        guest: sides::guest(&surface.types, &methods, &wit, &hooks)?,
        host: sides::host(&surface.types, &methods, &wit, &hooks, &servers)?,
        wit,
        sources: surface.types.index.read.take(),
        unbound: surface
            .unbound
            .iter()
            .map(|u| format!("{}::{}: {}", u.owner, u.name, u.why))
            .collect(),
    })
}

/// `horizon-mod-api`'s Cargo version, the one version of the mod API: the WIT package carries it,
/// so a `mod.wasm` imports the interfaces of the API version it was built against.
fn api_version(root: &Path) -> Result<String, String> {
    let path = root.join("crates/horizon-mod-api/Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: toml::Table = text
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    manifest
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: package.version missing", path.display()))
}

/// Each imported trait with the imported traits whose objects also serve it: itself first, then
/// its subtraits (a server host serves a call that asks for a host).
fn servers(types: &mut Types<'_>) -> Result<Vec<(&'static str, Vec<&'static str>)>, String> {
    let api = types.index.module("mod_api", &[])?;
    let mut supers: Vec<(&str, Vec<String>)> = Vec::new();
    for owner in IMPORTS {
        let path: Vec<String> = owner.split("::").map(str::to_string).collect();
        let Found::Item(id) = types.index.resolve_path(api, &path)? else {
            return Err(format!("{owner} is not a mod_api trait"));
        };
        let Item::Trait(item) = types.index.item(id) else {
            return Err(format!("{owner} is not a trait"));
        };
        let names = item
            .supertraits
            .iter()
            .filter_map(|bound| match bound {
                syn::TypeParamBound::Trait(t) => Some(t.path.segments.last()?.ident.to_string()),
                _ => None,
            })
            .collect();
        supers.push((owner, names));
    }
    let short = |owner: &str| owner.rsplit("::").next().unwrap().to_string();
    let serves = |sub: &str, of: &str| {
        let mut stack = vec![sub.to_string()];
        while let Some(t) = stack.pop() {
            if t == short(of) {
                return true;
            }
            if let Some((_, s)) = supers.iter().find(|(o, _)| short(o) == t) {
                stack.extend(s.iter().cloned());
            }
        }
        false
    };
    Ok(IMPORTS
        .iter()
        .map(|of| {
            let mut list = vec![*of];
            list.extend(
                IMPORTS
                    .iter()
                    .filter(|sub| *sub != of && serves(&short(sub), of)),
            );
            (*of, list)
        })
        .collect())
}

/// Each exported trait with the exported traits a handler of it also answers as: itself, then its
/// exported supertraits (a block entity part is a block part too).
fn views(types: &mut Types<'_>) -> Result<Vec<(String, Vec<String>)>, String> {
    let api = types.index.module("mod_api", &[])?;
    let mut out = Vec::new();
    for owner in exports() {
        let (path, _) = owner_parts(owner)?;
        let Found::Item(id) = types.index.resolve_path(api, &path)? else {
            return Err(format!("{owner} is not a mod_api trait"));
        };
        let Item::Trait(item) = types.index.item(id) else {
            return Err(format!("{owner} is not a trait"));
        };
        let mut list = vec![owner.to_string()];
        list.extend(item.supertraits.iter().filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(t) => {
                let name = t.path.segments.last()?.ident.to_string();
                exports().contains(&name.as_str()).then_some(name)
            }
            _ => None,
        }));
        out.push((owner.to_string(), list));
    }
    Ok(out)
}

/// The variants of the generated `HookId`, each with its hook's name.
fn hook_ids(types: &mut Types<'_>) -> Result<Vec<(String, String)>, String> {
    let api = types.index.module("mod_api", &[])?;
    let Found::Item(id) = types.index.resolve_path(api, &["HookId".to_string()])? else {
        return Err("HookId is not a mod_api enum".into());
    };
    let Item::Enum(item) = types.index.item(id) else {
        return Err("HookId is not an enum".into());
    };
    Ok(item
        .variants
        .iter()
        .map(|v| {
            (
                v.ident.to_string(),
                heck::ToSnakeCase::to_snake_case(v.ident.to_string().as_str()),
            )
        })
        .collect())
}

fn method(
    types: &mut Types<'_>,
    module: usize,
    owner: &'static str,
    function: &syn::TraitItemFn,
    subst: &[(String, Ty)],
) -> Result<Method, String> {
    let sig = &function.sig;
    if sig.generics.type_params().next().is_some() {
        return Err("generic".into());
    }
    let mut receiver = None;
    let mut params = Vec::new();
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(r) => receiver = Some(r.mutability.is_some()),
            FnArg::Typed(arg) => {
                let name = match &*arg.pat {
                    Pat::Ident(ident) => ident.ident.to_string(),
                    _ => format!("arg{}", params.len()),
                };
                let name = name.trim_start_matches('_').to_string();
                params.push((name, types.ty(module, &arg.ty, subst)?));
            }
        }
    }
    let ret = match &sig.output {
        ReturnType::Default => Ty::Unit,
        ReturnType::Type(_, ty) => types.ty(module, ty, subst)?,
    };
    Ok(Method {
        owner,
        name: sig.ident.to_string(),
        receiver,
        params,
        ret,
    })
}

fn hidden(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("doc")
            && quote::ToTokens::to_token_stream(attr)
                .to_string()
                .contains("hidden")
    })
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn print_surface() {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(print)
            .unwrap()
            .join()
            .unwrap();
    }
    fn print() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generated = super::super::sources(&root, super::super::FillMode::Ignore).unwrap();
        let included = [
            ("mod_api_generated.rs", generated.hooks.as_str()),
            ("mod_api_generated_host.rs", generated.host.as_str()),
            ("native_render_generated.rs", generated.render.as_str()),
        ];
        let wasm = super::generate(&root, &included).unwrap();
        for u in &wasm.unbound {
            println!("UNBOUND {u}");
        }
        let text = wasm.wit;
        std::fs::write("/box/work/wasm-spike/survey/mod-api.wit", &text).unwrap();
        std::fs::write("/box/work/wasm-spike/survey/guest.rs", &wasm.guest).unwrap();
        std::fs::write("/box/work/wasm-spike/survey/host.rs", &wasm.host).unwrap();
        let mut resolve = wit_parser::Resolve::default();
        resolve.push_str("mod-api.wit", &text).unwrap();
    }
}
