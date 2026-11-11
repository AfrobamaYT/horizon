//! Loader-layer proof using records exported by the generator's production test.
use super::*;
use crate::native_mods::{api, internals::Catalog, slots};
use horizon_mod_package::{
    manifest::inject::{Access, Inject, Kind, Route},
    Manifest,
};
use std::{fs, process::Command};

#[test]
#[ignore]
fn production_port_listeners_attach_and_invoke_on_both_routes() {
    let input = std::env::var_os("HORIZON_LISTED_PROOF_OUT")
        .expect("run generator production_listed_records with HORIZON_LISTED_PROOF_OUT first");
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    assert_eq!(rows.len(), 4);
    let dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/listed-port-loader");
    fs::create_dir_all(&dir).unwrap();
    crate::native_mods::tests::write_bundle(&dir, "example", "");
    let base = fs::read_to_string(dir.join(horizon_mod_package::manifest::MANIFEST)).unwrap();
    for row in rows {
        let original: Site = serde_json::from_value(row["site"].clone()).unwrap();
        let enclosing = row["enclosing"].clone();
        let name = &original.name;
        let canonical = original.export();
        let (args, callback, invocation) = if original.once {
            (
                "Args",
                "args.stack.0 += 1;",
                r#"
                let mut state = 0; let mut receiver = crate::game::Game(&mut state);
                let args = __SITEArgs { pos: (1,2,3), facing: crate::geometry::Direction, stack: crate::items::ItemStack(41) };
                let mut next = |_: &mut crate::game::Game<'_>, args: __SITEArgs| { assert_eq!(args.stack.0, 42); Ok(Some(args.stack)) };
                let result = f(&mut receiver, args, __SITEOriginal::new(&mut next)).unwrap().unwrap();
                assert_eq!(result.0, 42);
            "#,
            )
        } else if original.call.ends_with("farmland") {
            (
                "Args<'_, '_>",
                "assert_eq!(*args.w.value, 7); args.p[0] += 1;",
                r#"
                let value = 7; let world = crate::mobs::MobWorld { value: &value };
                let mut args = __SITEArgs { w: &world, p: [41, 0, 0] };
                let result = f((), &mut args, &mut |(), args| { assert_eq!(args.p[0], 42); true });
                assert!(result);
            "#,
            )
        } else {
            (
                "Args<'_>",
                "args.wood = \"changed\";",
                r#"
                let text = String::from("oak");
                let mut args = __SITEArgs { folder: "signs", wood: &text };
                let result = f((), &mut args, &mut |(), args| format!("{}:{}", args.folder, args.wood));
                assert_eq!(result, "signs:changed");
                assert_eq!(text, "oak");
            "#,
            )
        };
        // The callback text is generated once, then compiled unchanged for both routes.
        let callback = format!("use crate::mod_0_internals::raw::{{__{canonical}Args as Args, __{canonical}Receiver as Receiver}}; pub fn hook(_: Receiver<'_, '_>, args: &mut {args}) {{ {callback} }}");
        for private in [false, true] {
            let mut site = original.clone();
            site.private = private;
            let mut manifest = Manifest::parse(&base).unwrap();
            manifest.inject.push(Inject {
                local: None,
                local_type: None,
                position: None,
                expr: None,
                id: "port".into(),
                at: site.target.clone(),
                kind: Kind::BeforeCall,
                with: "hook".into(),
                access: Access::Internals,
                priority: 0,
                hash: Some(enclosing["hash"].as_str().unwrap().into()),
                call: Some(site.call.clone()),
                nth: Some(site.ordinal),
                ctx: Some(site.ctx.clone()),
            });
            fs::write(
                dir.join("internals.json"),
                serde_json::to_vec(&serde_json::json!([enclosing])).unwrap(),
            )
            .unwrap();
            let selected = if private {
                serde_json::json!([])
            } else {
                serde_json::json!([row["site"]])
            };
            fs::write(
                dir.join("points.json"),
                serde_json::to_vec(&serde_json::json!({"points": [], "sites": selected})).unwrap(),
            )
            .unwrap();
            let source = if private {
                serde_json::json!([row["site"]])
            } else {
                serde_json::json!([])
            };
            fs::write(
                dir.join("source-sites.json"),
                serde_json::to_vec(&source).unwrap(),
            )
            .unwrap();
            let catalog = Catalog::read(&dir).unwrap();
            assert_eq!(catalog.sites[0].private, private);
            let routes = slots::routes(&catalog, &manifest).unwrap();
            assert!(if private {
                matches!(routes.as_slice(), [Route::Source(_)])
            } else {
                matches!(routes.as_slice(), [Route::Slot(_)])
            });
            if site.target.contains("::game::") {
                let mut nominal = manifest.clone();
                nominal.inject[0].at = site
                    .target
                    .replace("::sign_edit", "")
                    .replace("::machines", "");
                assert!(matches!(
                    slots::routes(&catalog, &nominal),
                    Err(horizon_mod_package::manifest::inject::InjectError::Missing { .. })
                ));
            }
            let registration = api::main_rs("/* MOD_SLOTS */", &[&manifest], &catalog);
            let (mut exports, mut constructors, mut crates) = (
                std::collections::BTreeSet::new(),
                std::collections::BTreeSet::new(),
                std::collections::BTreeSet::new(),
            );
            site.facade(&mut exports, &mut constructors, &mut crates);
            let raw = row["raw"].as_str().unwrap();
            let mut module = format!(
                "{raw}\npub fn invoke(f: __{name}Listener) {{ {} }}",
                invocation.replace("__SITE", &format!("__{name}"))
            );
            for segment in site
                .listener
                .rsplit_once("::")
                .unwrap()
                .0
                .split("::")
                .skip(1)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                module = format!("pub mod {segment} {{ {module} }}");
            }
            // Engine stand-ins supply only the referents/owned values; records, aliases,
            // capabilities, facades, route selection and adapters are production output.
            let module = module
                .replace(
                    "pub mod game {",
                    "pub mod game { pub struct Game<'g>(pub &'g mut usize);",
                )
                .replace(
                    "pub mod mobs {",
                    "pub mod mobs { pub struct MobWorld<'g> { pub value: &'g usize }",
                );
            let aliases = if site.call.ends_with("farmland") {
                "pub mod game { pub struct Game<'g>(pub &'g mut usize); }"
            } else {
                "pub mod mobs {}"
            };
            let invoke_path = site.listener.rsplit_once("::").unwrap().0;
            let program = format!(
                r#"
extern crate self as horizon_client; extern crate self as horizon_client_lib;
extern crate self as horizon_client_render; extern crate self as horizon_core;
extern crate self as horizon_items; extern crate self as mod_points;
pub mod geometry {{ pub struct Direction; }} pub mod items {{ pub struct ItemStack(pub usize); }}
{aliases}
pub enum SiteId {{ {name} }} impl SiteId {{ pub const fn place(self) -> Self {{ self }} }}
pub mod __private {{ pub struct SlotListener {{ pub f: *const () }} impl SlotListener {{ pub const unsafe fn site(_: &'static str, _: super::SiteId, _: i32, _: &'static str, f: *const ()) -> Self {{ Self {{ f }} }} }} }}
{module}
mod mod_0_internals {{ pub mod raw {{ {exports} {constructors} }} }}
mod mod_0 {{ {callback} }}
fn main() {{ let listeners = [{registration}]; let f = unsafe {{ std::mem::transmute(listeners[0].f) }}; {invoke_path}::invoke(f); }}
"#,
                exports = exports.into_iter().collect::<String>(),
                constructors = constructors.into_iter().collect::<String>()
            );
            let path = dir.join("proof.rs");
            let binary = dir.join("proof");
            fs::write(&path, program).unwrap();
            let output = Command::new("rustc")
                .args(["--edition=2024", "-Awarnings"])
                .arg(&path)
                .arg("-o")
                .arg(&binary)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}: {}",
                site.call,
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(Command::new(&binary).status().unwrap().success());
            let mut stale = manifest.clone();
            stale.inject[0].hash = Some("stale".into());
            let error = slots::routes(&catalog, &stale).unwrap_err().to_string();
            assert!(error.contains(&site.target), "{error}");
        }
    }
    fs::remove_dir_all(dir).unwrap();
}
