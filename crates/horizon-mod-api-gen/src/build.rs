//! Cargo-facing generation and the launcher's trait injection boundary.
use super::{Catalog, Target, free_function_source, sources};
use quote::ToTokens;
use std::{collections::HashSet, fs, path::Path};

/// Generate only this crate's files, tracking every source read by the generator.
/// `root` is supplied at run time by the consuming crate's build script.
pub fn generate(root: &Path, out_dir: &Path, target: Target) -> Result<(), String> {
    track(root)?;
    let generated = sources(root, super::FillMode::Require)?;
    let outputs = match target {
        Target::Client => {
            let points = super::points::game_points(root)?;
            let list = serde_json::to_string_pretty(&points).map_err(|e| e.to_string())?;
            // The list the launcher routes injections by: the prebuilt game carries it.
            fs::write(out_dir.join("points.json"), list + "\n").map_err(|e| e.to_string())?;
            vec![
                ("mod_api_host_generated.rs", generated.implementation),
                ("mod_api_fire_generated.rs", generated.fire),
                ("points_generated.rs", super::points::slots(&points)),
            ]
        }
        Target::Api => {
            println!("cargo:rerun-if-env-changed=HORIZON_MOD_API_INJECTIONS");
            let hooks = if let Some(path) = std::env::var_os("HORIZON_MOD_API_INJECTIONS") {
                let path = Path::new(&path);
                println!("cargo:rerun-if-changed={}", path.display());
                let injections =
                    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
                inject(&generated.hooks, &injections)?
            } else {
                generated.hooks
            };
            let included = [
                ("mod_api_generated.rs", hooks.as_str()),
                ("mod_api_generated_host.rs", generated.host.as_str()),
                ("native_render_generated.rs", generated.render.as_str()),
            ];
            // The resolver recurses through the API's types: give it a deep stack.
            let wasm = std::thread::scope(|scope| {
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn_scoped(scope, || super::wasm::generate(root, &included))
                    .map_err(|e| e.to_string())?
                    .join()
                    .map_err(|_| "generating the wasm interface panicked".to_string())?
            })?;
            for path in &wasm.sources {
                println!("cargo:rerun-if-changed={}", path.display());
            }
            fs::write(out_dir.join("mod-api.wit"), &wasm.wit).map_err(|e| e.to_string())?;
            let points = super::points::records(&super::points::game_points(root)?);
            vec![
                ("points_api_generated.rs", points),
                ("mod_api_generated.rs", hooks),
                ("mod_api_generated_host.rs", generated.host),
                ("native_render_generated.rs", generated.render),
                ("capability_providers_generated.rs", wasm.providers),
                ("wasm_guest_generated.rs", wasm.guest),
                ("wasm_host_generated.rs", wasm.host),
            ]
        }
    };
    for (name, code) in outputs {
        fs::write(out_dir.join(name), &code).map_err(|e| e.to_string())?;
        syn::parse_file(&code).map_err(|e| format!("generated Rust: {name}: {e}"))?;
    }
    Ok(())
}

pub(super) fn track(root: &Path) -> Result<(), String> {
    fn directory(path: &Path, extension: &str, recursive: bool) -> Result<(), String> {
        // Track the directory as well: adding a helper source or fragment changes the inputs.
        println!("cargo:rerun-if-changed={}", path.display());
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() && recursive {
                directory(&path, extension, recursive)?;
            } else if path.extension().is_some_and(|ext| ext == extension) {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
        Ok(())
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join("crates/horizon-mod-api/src/lib.rs").display()
    );
    for retired in super::catalog::RETIRED {
        // Tracked while absent too: a file that appears must fail the next build.
        println!("cargo:rerun-if-changed={}", root.join(retired).display());
    }
    directory(&root.join(super::catalog::DECLARATIONS), "rs", false)?;
    directory(&root.join("crates/horizon-client/src/game"), "rs", true)?;
    println!(
        "cargo:rerun-if-changed={}",
        root.join("crates/horizon-client/src/game.rs").display()
    );
    for hook in Catalog::load(root)?.hooks {
        if let Some((krate, path)) = free_function_source(&hook.target) {
            let (module, _) = path.rsplit_once("::").ok_or("target missing module")?;
            let path = root
                .join("crates")
                .join(krate)
                .join("src")
                .join(format!("{}.rs", module.replace("::", "/")));
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Injection {
    name: String,
    event: String,
    declaration: String,
    event_declaration: String,
}

/// Add the launcher's trait methods and event structs to the generated catalog API.
/// The JSON file is the boundary: parse declarations once, then work with typed Rust items.
pub fn inject(code: &str, injections: &str) -> Result<String, String> {
    let injections: Vec<Injection> =
        serde_json::from_str(injections).map_err(|e| format!("trait injections: {e}"))?;
    if injections.is_empty() {
        return Ok(code.to_owned());
    }
    let mut file = syn::parse_file(code).map_err(|e| format!("generated API: {e}"))?;
    let mut events: HashSet<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item.ident.to_string()),
            syn::Item::Trait(item) => Some(item.ident.to_string()),
            _ => None,
        })
        .collect();
    let hooks = file
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == "Hooks" => Some(item),
            _ => None,
        })
        .ok_or("generated API missing Hooks trait")?;
    let mut methods: HashSet<_> = hooks
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
            _ => None,
        })
        .collect();
    let mut structs = Vec::new();
    let mut injected_ids = Vec::new();
    for injection in injections {
        let method: syn::TraitItemFn = syn::parse_str(&injection.declaration)
            .map_err(|e| format!("trait injection {}: {e}", injection.name))?;
        let event: syn::ItemStruct = syn::parse_str(&injection.event_declaration)
            .map_err(|e| format!("trait injection {}: {e}", injection.event))?;
        if method.sig.ident != injection.name || event.ident != injection.event {
            return Err("trait injection declaration does not match its name".into());
        }
        if !methods.insert(injection.name.clone()) {
            return Err(format!(
                "duplicate generated trait method {}",
                injection.name
            ));
        }
        if !events.insert(injection.event.clone()) {
            return Err(format!("duplicate generated event {}", injection.event));
        }
        let name = method.sig.ident.clone();
        let variant = syn::Ident::new(
            event
                .ident
                .to_string()
                .strip_suffix("Event")
                .ok_or("injected event must end in Event")?,
            event.ident.span(),
        );
        injected_ids.push((name, variant));
        hooks.items.push(syn::TraitItem::Fn(method));
        structs.push(syn::Item::Struct(event));
    }
    let ids = file
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Enum(item) if item.ident == "HookId" => Some(item),
            _ => None,
        })
        .ok_or("generated API missing HookId")?;
    for (_, variant) in &injected_ids {
        if ids
            .variants
            .iter()
            .any(|existing| existing.ident == *variant)
        {
            return Err(format!("duplicate generated hook id {variant}"));
        }
        ids.variants.push(syn::parse_quote!(#variant));
    }
    let count = ids.variants.len();
    let constants = file
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Mod(item) if item.ident == "hook_ids" => {
                item.content.as_mut().map(|(_, items)| items)
            }
            _ => None,
        })
        .ok_or("generated API missing hook_ids")?;
    for (name, variant) in injected_ids {
        constants
            .push(syn::parse_quote!(pub const #name: Option<super::HookId> = Some(super::HookId::#variant);));
    }
    let hook_count = file
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == "HOOK_COUNT" => Some(item),
            _ => None,
        })
        .ok_or("generated API missing HOOK_COUNT")?;
    *hook_count.expr = syn::parse_quote!(#count);
    file.items.extend(structs);
    Ok(file.into_token_stream().to_string())
}
