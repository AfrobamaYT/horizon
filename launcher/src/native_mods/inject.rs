//! Trait-backed injections, prepared as build-script input alongside the staged game.
use super::{Bundle, MANIFEST};
use horizon_mod_api_gen::{TraitContract as Contract, TraitInjection as Injection};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

/// Replace trait injections with ordinary mixin helpers, leaving ordinary injections unchanged.
pub(super) fn prepare(
    mods: &[Bundle],
    source: &Path,
    dispatch_order: &[Bundle],
) -> Result<Vec<Injection>, String> {
    let mut result = Vec::new();
    let mut names = HashSet::new();
    let owners: HashMap<_, _> = dispatch_order
        .iter()
        .enumerate()
        .map(|(i, b)| (b.manifest.id.as_str(), i))
        .collect();
    // These are the copied bundles under mods.incoming, never the player's originals.
    for bundle in mods {
        let path = bundle.dir.join(MANIFEST);
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut manifest: toml::Table = text
            .parse()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let entries = manifest
            .get_mut("injection")
            .and_then(toml::Value::as_array_mut);
        if let Some(entries) = entries {
            for (index, entry) in entries.iter_mut().enumerate() {
                let table = entry.as_table_mut().ok_or("injection must be a table")?;
                let Some(method) = table.remove("trait_method") else {
                    continue;
                };
                let target = if let Some(hook) = table.get("hook").and_then(toml::Value::as_str) {
                    horizon_mod_api_gen::hook_points(source)?
                        .into_iter()
                        .find(|point| point.name == hook)
                        .map(|point| point.target)
                        .ok_or_else(|| format!("unknown catalog hook {hook}"))?
                } else {
                    table
                        .get("target")
                        .and_then(toml::Value::as_str)
                        .ok_or("trait_method needs a target or catalog hook")?
                        .to_owned()
                };
                let target_crate = target.split("::").next().unwrap_or_default();
                if target_crate != "horizon_client" {
                    return Err(format!(
                        "trait injection target {target:?} is in crate {target_crate:?}; only horizon-client targets are supported"
                    ));
                }
                if table.contains_key("helper") {
                    return Err(format!(
                        "{}: trait_method cannot also specify helper",
                        path.display()
                    ));
                }
                let method = method
                    .as_str()
                    .ok_or("trait_method must be a Rust identifier")?;
                let _: syn::Ident =
                    syn::parse_str(method).map_err(|_| "trait_method must be a Rust identifier")?;
                let name = format!("inject_{}_{}", bundle.manifest.id.replace('-', "_"), method);
                let ident: syn::Ident =
                    syn::parse_str(&name).map_err(|_| "invalid generated trait method")?;
                if !names.insert(name.clone()) {
                    return Err(format!("duplicate generated trait method {name}"));
                }
                let cancellable = match table.get("cancellable") {
                    None => false,
                    Some(v) => v.as_bool().ok_or("cancellable must be a boolean")?,
                };
                let mut parameters = Vec::new();
                let mut seen = HashSet::new();
                let mut value = None;
                if let Some(bindings) = table.get("bindings") {
                    for binding in bindings.as_array().ok_or("bindings must be an array")? {
                        let binding = binding.as_table().ok_or("binding must be a table")?;
                        let name = binding
                            .get("name")
                            .and_then(toml::Value::as_str)
                            .ok_or("binding needs a name")?;
                        let ident: syn::Ident =
                            syn::parse_str(name).map_err(|_| "invalid binding name")?;
                        if !seen.insert(name.to_owned()) {
                            return Err(format!("duplicate binding {name}"));
                        }
                        let ty = binding
                            .get("ty")
                            .and_then(toml::Value::as_str)
                            .ok_or("binding needs a type")?;
                        let ty: syn::Type =
                            syn::parse_str(ty).map_err(|e| format!("invalid binding type: {e}"))?;
                        if binding.get("value").and_then(toml::Value::as_str) == Some("$value") {
                            value = Some(ident.clone());
                        }
                        parameters.push((ident, ty));
                    }
                }
                let kind = table
                    .get("kind")
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default();
                let contract = match (kind, cancellable) {
                    ("HEAD", false) => Contract::Listen,
                    ("HEAD", true) => Contract::Cancel,
                    ("RETURN" | "MODIFY_ARG" | "MODIFY_EXPRESSION_VALUE", false) => {
                        Contract::Value(value.ok_or_else(|| {
                            format!(
                                "{}: a {kind} trait_method needs a `$value` binding",
                                path.display()
                            )
                        })?)
                    }
                    (_, true) => {
                        return Err(format!(
                            "{}: cancellable trait_method needs HEAD",
                            path.display()
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "{}: trait_method supports HEAD, RETURN, MODIFY_ARG and MODIFY_EXPRESSION_VALUE",
                            path.display()
                        ));
                    }
                };
                let injection = Injection::new(
                    ident,
                    format!(
                        "Injection at `{target}` ({kind}), from mod {}.",
                        bundle.manifest.id
                    ),
                    parameters,
                    contract,
                );
                let helper_path = format!("horizon-trait-injection-{index}.rs");
                let helper_file = bundle.dir.join(&helper_path);
                if helper_file.exists() {
                    return Err(format!(
                        "{} is reserved for generated helpers",
                        helper_file.display()
                    ));
                }
                fs::write(
                    &helper_file,
                    injection.helper(owners[bundle.manifest.id.as_str()]),
                )
                .map_err(|e| e.to_string())?;
                table.insert("helper".into(), helper_path.into());
                result.push(injection);
            }
        }
        fs::write(
            &path,
            toml::to_string(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(result)
}

pub(super) const INPUT: &str = ".horizon-mod-api-injections.json";
pub(super) const ENV: &str = "HORIZON_MOD_API_INJECTIONS";

/// Supply declarations to the generator, which checks conflicts against the catalog at build time.
/// This file travels with cached mixin output; generated Rust lives only in Cargo's OUT_DIR.
pub(super) fn stage(stage: &Path, injections: &[Injection]) -> Result<(), String> {
    let declarations = injections.iter().map(Injection::input).collect::<Vec<_>>();
    let path = stage.join(INPUT);
    fs::write(
        &path,
        serde_json::to_vec(&declarations).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str, injection: &str) -> (std::path::PathBuf, Bundle) {
        let root = super::super::tests::dir(&format!("inject-{name}"));
        super::super::tests::write_bundle(&root, "example", "");
        let path = root.join(horizon_mods::manifest::MANIFEST);
        let text = fs::read_to_string(&path).unwrap()
            + &injection.replace("[[injection]]", "[[native.injection]]");
        fs::write(path, text).unwrap();
        let bundle = super::super::bundle_in(&root).unwrap().unwrap();
        (root, bundle)
    }

    fn prepare(mods: &[Bundle], source: &Path) -> Result<Vec<Injection>, String> {
        for bundle in mods {
            super::super::package::tool_manifest(bundle)?;
        }
        super::prepare(mods, source, mods)
    }

    fn compile_and_run(root: &Path, program: &str) {
        let source = root.join("proof.rs");
        let binary = root.join(format!("proof-bin{}", std::env::consts::EXE_SUFFIX));
        fs::write(&source, program).unwrap();
        let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let compiled = std::process::Command::new(compiler)
            .args(["--edition", "2024"])
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        assert!(std::process::Command::new(binary)
            .status()
            .unwrap()
            .success());
    }

    #[test]
    fn a_module_stages_with_a_helper_that_calls_into_it() {
        let (root, _bundle) = fixture(
            "module",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'HEAD'\nhelper = 'src/lib.rs'\nbindings = []\n",
        );
        // The helper calls into the module; both must reach the tool together.
        fs::write(
            root.join("src/lib.rs"),
            "pub fn tick_hook() { crate::runtime::bump(&mut 0); }\n",
        )
        .unwrap();
        fs::write(
            root.join("src/runtime.rs"),
            "pub(crate) fn bump(value: &mut i32) { *value += 1; }\n",
        )
        .unwrap();
        let path = root.join(horizon_mods::manifest::MANIFEST);
        let text = fs::read_to_string(&path).unwrap()
            + "[[native.module]]\nid = 'runtime'\nfile = 'crates/horizon-client/src/game.rs'\nname = 'runtime'\nsource = 'src/runtime.rs'\n";
        fs::write(&path, text).unwrap();
        let bundle = super::super::bundle_in(&root).unwrap().unwrap();
        let mixin_input = root.with_extension("mixins");
        let tool_mods =
            super::super::stage_mixins(std::slice::from_ref(&bundle), &mixin_input).unwrap();
        assert_eq!(tool_mods.len(), 1);
        let staged = &tool_mods[0].dir;
        let manifest = fs::read_to_string(staged.join(MANIFEST)).unwrap();
        assert!(manifest.contains("[[module]]"));
        assert!(manifest.contains("name = \"runtime\""));
        assert!(manifest.contains("source = \"src/runtime.rs\""));
        // `visibility` stays out of the tool manifest until a package sets it.
        assert!(!manifest.contains("visibility"));
        assert_eq!(
            fs::read_to_string(staged.join("src/runtime.rs")).unwrap(),
            "pub(crate) fn bump(value: &mut i32) { *value += 1; }\n"
        );
        assert!(staged.join("src/lib.rs").is_file());
    }

    #[test]
    fn a_module_only_bundle_still_needs_the_tool() {
        let root = super::super::tests::dir("inject-module-only");
        super::super::tests::write_bundle(&root, "example", "");
        let path = root.join(horizon_mods::manifest::MANIFEST);
        let text = fs::read_to_string(&path).unwrap()
            + "[[native.module]]\nid = 'runtime'\nfile = 'crates/horizon-client/src/game.rs'\nname = 'runtime'\nsource = 'src/lib.rs'\n";
        fs::write(&path, text).unwrap();
        let bundle = super::super::bundle_in(&root).unwrap().unwrap();
        let mixin_input = root.with_extension("mixins");
        let tool_mods =
            super::super::stage_mixins(std::slice::from_ref(&bundle), &mixin_input).unwrap();
        assert_eq!(tool_mods.len(), 1);
        assert!(tool_mods[0].dir.join(MANIFEST).is_file());
    }

    #[test]
    fn owner_dispatch_and_cancel_use_full_order_after_tool_filtering() {
        let (root, bundle) = fixture(
            "cancel",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'HEAD'\ntrait_method = 'tick'\ncancellable = true\nbindings = [{ name = 'amount', ty = '&mut i32', value = '&mut *amount' }]\n",
        );
        let original = fs::read(root.join(horizon_mods::manifest::MANIFEST)).unwrap();
        let plain_root = super::super::tests::dir("inject-cancel-plain");
        super::super::tests::write_bundle(&plain_root, "plain", "");
        let plain = super::super::bundle_in(&plain_root).unwrap().unwrap();
        let dispatch_order = [plain, bundle];
        let mixin_input = root.with_extension("mixins");
        let tool_mods = super::super::stage_mixins(&dispatch_order, &mixin_input).unwrap();
        assert_eq!(tool_mods.len(), 1);
        assert_eq!(tool_mods[0].manifest.id, "example");
        assert!(!mixin_input.join("plain").exists());
        let injections = super::prepare(&tool_mods, &root, &dispatch_order).unwrap();
        let helper =
            fs::read_to_string(tool_mods[0].dir.join("horizon-trait-injection-0.rs")).unwrap();
        let manifest = fs::read_to_string(tool_mods[0].dir.join(MANIFEST)).unwrap();
        assert_eq!(
            fs::read(root.join(horizon_mods::manifest::MANIFEST)).unwrap(),
            original
        );
        assert!(dispatch_order.iter().all(|bundle| {
            !bundle.dir.join(MANIFEST).exists()
                && !bundle.dir.join("horizon-trait-injection-0.rs").exists()
        }));
        assert!(!manifest.contains("trait_method"));
        assert!(!manifest.contains("[api]"));
        let stage_dir = root.join("stage");
        fs::create_dir_all(&stage_dir).unwrap();
        stage(&stage_dir, &injections).unwrap();
        assert!(!stage_dir.join("crates").exists());
        let output = horizon_mod_api_gen::inject(
            "#[derive(Clone, Copy)] pub enum HookId {} pub(crate) const HOOK_COUNT: usize = 0; pub mod hook_ids {} pub trait Hooks: Send + Sync {\n}\n",
            &fs::read_to_string(stage_dir.join(INPUT)).unwrap(),
        )
        .unwrap();
        let file = syn::parse_file(&output).unwrap();
        // `horizon-mod-api` denies missing docs: the generated method and every event field need one.
        let documented = |attrs: &[syn::Attribute]| attrs.iter().any(|a| a.path().is_ident("doc"));
        for item in &file.items {
            match item {
                syn::Item::Trait(hooks) if hooks.ident == "Hooks" => {
                    for member in &hooks.items {
                        if let syn::TraitItem::Fn(method) = member {
                            assert!(documented(&method.attrs), "{} has no doc", method.sig.ident);
                        }
                    }
                }
                syn::Item::Struct(event) if event.ident == "InjectExampleTickEvent" => {
                    assert!(documented(&event.attrs));
                    assert!(event.fields.iter().all(|field| documented(&field.attrs)));
                }
                _ => {}
            }
        }
        assert!(
            output.contains("Injection at `horizon_client::game::tick` (HEAD), from mod example.")
        );
        let program = format!(
            r#"
            mod mod_api {{
                use std::sync::OnceLock;
                #[derive(PartialEq)] pub enum Flow {{ Continue, Cancel }}
                {output}
                pub static SUBSCRIBED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                pub fn subscribed_by(_: usize, _: Option<HookId>) -> bool {{ SUBSCRIBED.load(std::sync::atomic::Ordering::Relaxed) }}
                struct Owner;
                impl Hooks for Owner {{
                    fn inject_example_tick(&self, event: &mut InjectExampleTickEvent) -> Flow {{
                        event.amount = event.amount * 2 + 40;
                        Flow::Cancel
                    }}
                }}
                struct Other;
                impl Hooks for Other {{
                    fn inject_example_tick(&self, _: &mut InjectExampleTickEvent) -> Flow {{ panic!("wrong owner") }}
                }}
                pub static ENTERED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
                pub struct ModEntry {{ pub id: &'static str, pub hooks: Box<dyn Hooks> }}
                impl ModEntry {{
                    pub fn enter(&self, hook: &'static str) {{ ENTERED.lock().unwrap().push(format!("{{}}:{{hook}}", self.id)); }}
                }}
                pub fn mods() -> &'static [ModEntry] {{
                    static MODS: OnceLock<Vec<ModEntry>> = OnceLock::new();
                    MODS.get_or_init(|| vec![ModEntry {{ id: "plain", hooks: Box::new(Other) }}, ModEntry {{ id: "example", hooks: Box::new(Owner) }}, ModEntry {{ id: "other", hooks: Box::new(Other) }}])
                }}
                pub mod __private {{ pub use super::mods; }}
            }}
            {helper}
            fn main() {{
                let mut value = 1;
                assert!(matches!(horizon_trait_dispatch(&mut value), core::ops::ControlFlow::Continue(())));
                assert_eq!(value, 1);
                mod_api::SUBSCRIBED.store(true, std::sync::atomic::Ordering::Relaxed);
                assert!(matches!(horizon_trait_dispatch(&mut value), core::ops::ControlFlow::Break(())));
                assert_eq!(value, 42);
                assert_eq!(*mod_api::ENTERED.lock().unwrap(), ["example:inject_example_tick"]);
            }}
        "#
        );
        compile_and_run(&root, &program);
        fs::remove_dir_all(mixin_input).unwrap();
        fs::remove_dir_all(plain_root).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn value_injection_returns_what_the_owner_changed() {
        let (root, bundle) = fixture(
            "value",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'RETURN'\ntrait_method = 'total'\nbindings = [{ name = 'total', ty = 'i32', value = '$value' }, { name = 'calls', ty = '&mut u32', value = '&mut *calls' }]\n",
        );
        let injections = prepare(&[bundle], &root).unwrap();
        let helper = fs::read_to_string(root.join("horizon-trait-injection-0.rs")).unwrap();
        let stage_dir = root.join("stage");
        fs::create_dir_all(&stage_dir).unwrap();
        stage(&stage_dir, &injections).unwrap();
        let output = horizon_mod_api_gen::inject(
            "#[derive(Clone, Copy)] pub enum HookId {} pub(crate) const HOOK_COUNT: usize = 0; pub mod hook_ids {} pub trait Hooks: Send + Sync {\n}\n",
            &fs::read_to_string(stage_dir.join(INPUT)).unwrap(),
        )
        .unwrap();
        let program = format!(
            r#"
            mod mod_api {{
                use std::sync::atomic::{{AtomicBool, Ordering}};
                use std::sync::OnceLock;
                {output}
                pub fn subscribed_by(_: usize, _: Option<HookId>) -> bool {{ true }}
                struct Owner;
                impl Hooks for Owner {{
                    fn inject_example_total(&self, event: &mut InjectExampleTotalEvent) {{
                        event.total = event.total * 2 + 1;
                        event.calls += 1;
                    }}
                }}
                struct Other;
                impl Hooks for Other {{
                    fn inject_example_total(&self, _: &mut InjectExampleTotalEvent) {{ panic!("wrong owner") }}
                }}
                pub static INSTALLED: AtomicBool = AtomicBool::new(false);
                pub struct ModEntry {{ pub id: &'static str, pub hooks: Box<dyn Hooks> }}
                impl ModEntry {{
                    pub fn enter(&self, _: &'static str) {{}}
                }}
                pub fn mods() -> &'static [ModEntry] {{
                    static MODS: OnceLock<Vec<ModEntry>> = OnceLock::new();
                    if !INSTALLED.load(Ordering::SeqCst) {{
                        return &[];
                    }}
                    MODS.get_or_init(|| vec![ModEntry {{ id: "owner", hooks: Box::new(Owner) }}, ModEntry {{ id: "other", hooks: Box::new(Other) }}])
                }}
                pub mod __private {{ pub use super::mods; }}
            }}
            {helper}
            fn main() {{
                let mut calls = 0;
                assert_eq!(horizon_trait_dispatch(20, &mut calls), 20);
                assert_eq!(calls, 0);
                mod_api::INSTALLED.store(true, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(horizon_trait_dispatch(20, &mut calls), 41);
                assert_eq!(calls, 1);
            }}
        "#
        );
        compile_and_run(&root, &program);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_input_checks_conflicts_and_clears_removed_injections() {
        let (root, bundle) = fixture(
            "conflicts",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'HEAD'\ntrait_method = 'tick'\n",
        );
        let injections = prepare(&[bundle], &root).unwrap();
        stage(&root, &injections).unwrap();
        let input = fs::read_to_string(root.join(INPUT)).unwrap();
        for existing in [
            "pub trait Hooks: Send + Sync { fn inject_example_tick(&self) {} }",
            "pub trait Hooks: Send + Sync {} pub struct InjectExampleTickEvent;",
        ] {
            assert!(horizon_mod_api_gen::inject(existing, &input).is_err());
        }
        stage(&root, &[]).unwrap();
        let input = fs::read_to_string(root.join(INPUT)).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&input).unwrap(),
            serde_json::json!([])
        );
        let output =
            horizon_mod_api_gen::inject("pub trait Hooks: Send + Sync {}", &input).unwrap();
        assert!(!output.contains("inject_example_tick"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn trait_edits_do_not_invalidate_mixin_cache() {
        let (root, bundle) = fixture(
            "cache",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'HEAD'\ntrait_method = 'tick'\n",
        );
        let key = || {
            super::super::inject_cache::key(
                "revision",
                "mixins",
                super::super::Profile::Performance,
                std::slice::from_ref(&bundle),
            )
            .unwrap()
        };
        let before = key();
        fs::write(root.join("src/lib.rs"), "// one line edit").unwrap();
        assert_eq!(before, key());
        fs::create_dir_all(root.join("data/example")).unwrap();
        fs::write(root.join("data/example/table.bin"), "mixin input").unwrap();
        assert_ne!(before, key());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mixin_inputs_inside_api_are_not_excluded_from_cache() {
        let (root, bundle) = fixture(
            "api-helper",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = 'HEAD'\nhelper = 'src/lib.rs'\n",
        );
        let key = || {
            super::super::inject_cache::key(
                "revision",
                "mixins",
                super::super::Profile::Performance,
                std::slice::from_ref(&bundle),
            )
            .unwrap()
        };
        let before = key();
        fs::write(root.join("src/lib.rs"), "fn changed() {}\n").unwrap();
        assert_ne!(before, key());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_manual_tool_crate_declarations() {
        let (root, _) = fixture("tool-route", "");
        let path = root.join(horizon_mods::manifest::MANIFEST);
        let text = fs::read_to_string(&path).unwrap();
        fs::write(path, text + "[[crate]]\nid='code'\npath='src'\n").unwrap();
        assert!(super::super::bundle_in(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_build_key_tracks_dispatch_order() {
        let (root, first) = fixture("order", "");
        let mut second = first.clone();
        second.manifest.id = "second".into();
        let a = super::super::version_for(
            "0123456789012345678901234567890123456789",
            "mixins",
            super::super::Profile::Performance,
            super::super::Target::Client,
            &[first.clone(), second.clone()],
        )
        .unwrap();
        let b = super::super::version_for(
            "0123456789012345678901234567890123456789",
            "mixins",
            super::super::Profile::Performance,
            super::super::Target::Client,
            &[second, first],
        )
        .unwrap();
        assert_ne!(a, b);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn trait_injections_reject_targets_in_other_crates() {
        let (root, bundle) = fixture(
            "other-crate",
            "[[injection]]\nid = 'hook'\ntarget = 'horizon_world::loot::load'\nkind = 'HEAD'\ntrait_method = 'load'\n",
        );
        let error = prepare(&[bundle], &root).err().unwrap();
        assert!(error.contains("horizon_world::loot::load") && error.contains("horizon_world"));
        fs::remove_dir_all(root).unwrap();

        let (root, bundle) = fixture(
            "other-catalog",
            "[[injection]]\nid = 'hook'\nhook = 'loot_table_load'\nkind = 'HEAD'\ntrait_method = 'load'\n",
        );
        let declarations = root.join(horizon_mod_api_gen::DECLARATIONS);
        fs::create_dir_all(&declarations).unwrap();
        fs::write(declarations.join("mod.rs"), "mod loot;").unwrap();
        fs::write(
            declarations.join("loot.rs"),
            r#"pub trait Hooks {
    /// Loot.
    #[hook(side = "server", contract = "listen", target = "horizon_world::loot::load")]
    fn loot_table_load();
}"#,
        )
        .unwrap();
        let error = prepare(&[bundle], &root).err().unwrap();
        assert!(error.contains("horizon_world::loot::load") && error.contains("horizon_world"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn trait_injection_kinds_are_checked() {
        for (kind, extra, expected) in [
            (
                "AFTER_CALL",
                "",
                "supports HEAD, RETURN, MODIFY_ARG and MODIFY_EXPRESSION_VALUE",
            ),
            (
                "RETURN",
                "bindings = [{ name = 'amount', ty = 'i32', value = 'amount' }]\n",
                "`$value` binding",
            ),
            (
                "MODIFY_ARG",
                "cancellable = true\nbindings = [{ name = 'points', ty = 'i32', value = '$value' }]\n",
                "cancellable trait_method needs HEAD",
            ),
        ] {
            let (root, bundle) = fixture(
                "kinds",
                &format!(
                    "[[injection]]\nid = 'hook'\ntarget = 'horizon_client::game::tick'\nkind = '{kind}'\ntrait_method = 'tick'\n{extra}"
                ),
            );
            let error = prepare(&[bundle], &root).err().unwrap();
            assert!(error.contains(expected), "{kind}: {error}");
            fs::remove_dir_all(root).unwrap();
        }
    }
}
