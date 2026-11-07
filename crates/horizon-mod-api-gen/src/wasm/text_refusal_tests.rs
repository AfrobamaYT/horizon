use super::*;
use std::{collections::BTreeSet, fs, process::Command};

#[test]
fn generated_text_refusals_are_typed_on_both_sides() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let generated = crate::sources(&root, crate::FillMode::Require).unwrap();
    let included = [
        ("mod_api_generated.rs", generated.hooks.as_str()),
        ("mod_api_generated_host.rs", generated.host.as_str()),
    ];
    let mut types = ty::Types::new(index::Index::new(&root, &included)).unwrap();
    let module = types.index.module("mod_api", &[]).unwrap();
    types.index.modules[module].items.extend(syn::parse_file("pub struct TextBatch { pub values: Vec<Option<Text>>, pub shape: [u32; 13] } pub enum TextFormError { TooDeep, Malformed, Invalid }").unwrap().items);
    let text = types
        .ty(module, &syn::parse_str("Text").unwrap(), &[])
        .unwrap();
    let batch = types
        .ty(module, &syn::parse_str("TextBatch").unwrap(), &[])
        .unwrap();
    let error = types
        .ty(module, &syn::parse_str("TextFormError").unwrap(), &[])
        .unwrap();
    let Ty::Named(text_id) = text else { panic!() };
    let Ty::Named(error_id) = error else { panic!() };
    // Isolate the adapter contract from TextForm's own tree validation. The real Text
    // name and Via rule are used; the fixture supplies independent refusal values.
    types.defs[text_id].kind = ty::Kind::Via(Ty::Prim("u32", "u32"));
    types.defs[error_id].kind =
        ty::Kind::Enum(vec!["TooDeep".into(), "Malformed".into(), "Invalid".into()]);
    let methods: Vec<_> = [
        (
            "direct",
            vec![("input".into(), text.clone())],
            Ty::Prim("u32", "u32"),
        ),
        (
            "batch",
            vec![("input".into(), batch)],
            Ty::Prim("u32", "u32"),
        ),
        ("output", vec![], text),
    ]
    .into_iter()
    .map(|(name, params, ok)| Method {
        via: None,
        owner: "TextProbe",
        name: name.into(),
        receiver: Some(true),
        params,
        ret: Ty::Result(Box::new(ok), Box::new(error.clone())),
    })
    .collect();
    // GuiDraw carries Text through a variant payload, not a direct method argument.
    let gui = types
        .ty(module, &syn::parse_str("GuiDraw").unwrap(), &[])
        .unwrap();
    let gui_method = Method {
        via: None,
        owner: "TextProbe",
        name: "gui_paint".into(),
        receiver: Some(true),
        params: vec![("draws".into(), Ty::List(Box::new(gui), ty::List::Vec))],
        ret: methods[0].ret.clone(),
    };
    wit::refuse(&mut types);
    assert!(
        sides::guest_call(&types, &gui_method, "paint", None)
            .unwrap()
            .contains("Err(Refused::Text(why))")
    );
    let (defs, _) = wit::reached(&types, &methods.iter().collect::<Vec<_>>());
    let conversions = glue::conversions(&types, &defs, &BTreeSet::new());
    let host = methods
        .iter()
        .map(|m| sides::host_import(&types, m, "self", false).unwrap())
        .collect::<String>();
    types.guest.set(true);
    let guest = methods
        .iter()
        .map(|m| {
            sides::guest_call(&types, m, &format!("super::imports::{}", m.name), None).unwrap()
        })
        .collect::<String>();
    let refused = fs::read_to_string(root.join("crates/horizon-mod-api/src/wasm_refused.rs"))
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = include_str!("text_refusal_fixture.rs")
        .replace("// REFUSED", &refused)
        .replace(
            "// GENERATED",
            &format!(
                r#"
mod host {{ use super::*; {conversions}
    pub trait ProbeCalls {{
        fn direct(&mut self, input: u32) -> Result<Result<u32, wire::TextFormError>>;
        fn batch(&mut self, input: wire::TextBatch) -> Result<Result<u32, wire::TextFormError>>;
        fn output(&mut self) -> Result<Result<u32, wire::TextFormError>>;
    }}
    impl ProbeCalls for Probe {{ {host} }}
}}
mod guest {{ use super::*; {conversions} pub struct Guest; impl TextProbe for Guest {{ {guest} }} }}
"#
            ),
        );
    let dir = root
        .join("target")
        .join(format!("text-refusal-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("fixture.rs");
    let binary = dir.join("fixture");
    fs::write(&src, source).unwrap();
    let built = Command::new("rustc")
        .args(["--edition=2024", "--test", "-Awarnings"])
        .arg(&src)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(binary).arg("--nocapture").output().unwrap();
    assert!(
        ran.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    println!("{}", String::from_utf8_lossy(&ran.stdout));
    fs::remove_dir_all(dir).unwrap();
}
