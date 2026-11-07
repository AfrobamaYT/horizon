//! Checked native source adapter inputs. Production staging is wired in U5b.
use horizon_mod_package::manifest::inject::{
    route, Access, Entry, Inject, InjectError, Kind, NoSlotReason, Place, Route,
};
use point_shape::{raw_able, shape};
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::Write;

struct Adapter {
    inject: Inject,
    reason: NoSlotReason,
    identity: String,
    kind: &'static str,
    code: TokenStream,
    listener: syn::Ident,
}

fn lower(
    entry: Entry<'_>,
    places: &[Place],
    signature: &syn::Signature,
    receiver: Option<&syn::Type>,
) -> Result<Option<Adapter>, InjectError> {
    let inject = entry.inject;
    let owner = entry.mod_id;
    let reason = match route(entry, places)? {
        Route::Slot(_) => return Ok(None),
        Route::Source(reason) => reason,
    };
    let invalid = |field, fix| InjectError::Invalid {
        id: format!("{owner}:{}", inject.id),
        field,
        fix,
    };
    if inject.access != Access::Internals {
        return Err(invalid("access", "API source adapters await U5f"));
    }
    let kind = match inject.kind {
        Kind::Head => "HEAD",
        Kind::Return => "RETURN",
        _ => {
            return Err(invalid(
                "kind",
                "this source adapter requires head or return",
            ))
        }
    };
    if !raw_able(signature, None) {
        return Err(invalid(
            "at",
            "source listeners require a non-generic signature",
        ));
    }
    // Length-prefixed UTF-8, not another signature hasher. Encoding every byte also keeps
    // the symbol distinct after the raw-record generator's CamelCase conversion.
    let mut identity = String::from("source");
    for part in [owner, inject.id.as_str(), inject.at.as_str(), kind] {
        for byte in (part.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(part.bytes())
        {
            write!(identity, "{byte:02x}").unwrap();
        }
    }
    let point = shape(quote!(raw, name = #identity), signature)
        .map_err(|_| invalid("at", "the source signature cannot form a raw record"))?;
    let items = point.raw_items(signature, receiver, None);
    let listener = if inject.kind == Kind::Head {
        point.raw_listener()
    } else {
        point.raw_return_listener()
    };
    Ok(Some(Adapter {
        inject: inject.clone(),
        reason,
        identity,
        kind,
        code: items,
        listener,
    }))
}

#[test]
fn checked_routes_and_identities_do_not_alias() {
    let make = |owner: &str, id: &str, at: &str, kind: &str, slots| {
        let manifest = declaration(owner, id, at, kind);
        lower_entry(&manifest, slots, "fn target(value: i32) -> i32").unwrap()
    };
    let original = make("one", "ab", "game::c", "head", vec![]).unwrap();
    assert_eq!(original.inject.with, "callback");
    assert_eq!(original.reason, NoSlotReason::NotListed);
    for (owner, id, at, kind) in [
        ("two", "ab", "game::c", "head"),
        ("one", "a", "bgame::c", "head"),
        ("one", "ab", "game::c", "return"),
    ] {
        assert_ne!(
            original.identity,
            make(owner, id, at, kind, vec![]).unwrap().identity
        );
    }
    assert!(make(
        "one",
        "ab",
        "game::c",
        "head",
        vec![horizon_mod_package::manifest::inject::SlotKind::Head]
    )
    .is_none());
    let mut manifest = declaration("one", "ab", "game::c", "head");
    manifest.inject[0].hash = "changed".into();
    assert!(matches!(
        lower_entry(&manifest, vec![], "fn target()"),
        Err(InjectError::Changed { .. })
    ));
    manifest.inject[0].hash = "fingerprint".into();
    manifest.inject[0].access = Access::Api;
    assert!(matches!(
        lower_entry(&manifest, vec![], "fn target()"),
        Err(InjectError::Invalid {
            field: "access",
            ..
        })
    ));
    manifest.inject[0].access = Access::Internals;
    manifest.inject[0].kind = Kind::ModifyArg;
    assert!(matches!(
        lower_entry(&manifest, vec![], "fn target()"),
        Err(InjectError::Invalid { field: "kind", .. })
    ));
}

fn declaration(owner: &str, id: &str, at: &str, kind: &str) -> horizon_mod_package::Manifest {
    horizon_mod_package::Manifest::parse(&format!(
        "id='{owner}'\nname='Source fixture'\nversion='0.1.0'\napi='0.2'\ncapabilities=['registries']\n[native]\ngame='26.3'\nhorizon='{}'\nmixins='{}'\n[[inject]]\nid='{id}'\nat='{at}'\nkind='{kind}'\nwith='callback'\naccess='internals'\nhash='fingerprint'\n", "0".repeat(40), "0".repeat(40)
    )).unwrap()
}

fn lower_entry(
    manifest: &horizon_mod_package::Manifest,
    slots: Vec<horizon_mod_package::manifest::inject::SlotKind>,
    signature: &str,
) -> Result<Option<Adapter>, InjectError> {
    lower(
        Entry {
            inject: &manifest.inject[0],
            target: horizon_mod_package::manifest::inject::Target::Native,
            mod_id: &manifest.id,
            resolved_call: None,
        },
        &[Place {
            at: manifest.inject[0].at.clone(),
            hash: "fingerprint".into(),
            slots,
            calls: vec![],
            native_only: false,
        }],
        &syn::parse_str(signature).unwrap(),
        Some(&syn::parse_quote!(Game)),
    )
}

#[test]
fn source_adapter_inputs_compile_and_write_back() {
    for (kind, signature, fields, body, setup, expected) in [
        ("head", "fn target(&mut self, value: i32) -> i32", "i32, i32", "*args.value += 2; *args.result = Some(9);", "let mut result = None; for _ in 0..2 { let mut args = RECORD { value: &mut value, result: &mut result }; call(&mut game, &mut args); } assert_eq!(result, Some(9));", "assert_eq!(value, 7);"),
        ("head", "fn target(&mut self, value: i32)", "i32", "*args.value += 2; *args.cancel = true;", "let mut cancel = false; for _ in 0..2 { let mut args = RECORD { value: &mut value, cancel: &mut cancel }; call(&mut game, &mut args); } assert!(cancel);", "assert_eq!(value, 7);"),
        ("return", "fn target(&mut self, value: i32) -> i32", "", "*args += 5;", "for _ in 0..2 { call(&mut game, &mut value); }", "assert_eq!(value, 13);"),
    ] {
        let adapter = lower_entry(&declaration("one", "test", "game::target", kind), vec![], signature).unwrap().unwrap();
        assert_eq!(adapter.kind, kind.to_uppercase());
        let record = format!("__{}Raw", point_shape::camel(&adapter.identity));
        let alias = adapter.listener;
        let arg = if kind == "return" { "i32".into() } else { format!("{record}<'_, {fields}>") };
        let code = format!(r#"
            pub struct Game {{ bias: i32 }}
            {generated}
            fn callback(game: &mut Game, args: &mut {arg}) {{ game.bias += 1; {body} }}
            fn main() {{ let call: {alias} = callback;
                let mut game = Game {{ bias: 0 }}; let mut value = 3; {setup} {expected} assert_eq!(game.bias, 2);
            }}
        "#, generated=adapter.code, setup=setup.replace("RECORD", &record));
        let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/source-adapter-fixture");
        std::fs::create_dir_all(&target).unwrap();
        let input = target.join("main.rs");
        let output = target.join("fixture");
        std::fs::write(&input, &code).unwrap();
        let status = std::process::Command::new("rustc").args(["--edition", "2024"]).arg(&input).arg("-o").arg(&output).status().unwrap();
        assert!(status.success(), "{code}");
        assert!(std::process::Command::new(&output).status().unwrap().success());
    }
}
