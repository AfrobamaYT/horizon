use super::*;

fn function(id: &str, refs: &[&str]) -> Function {
    Function::new(
        id,
        "",
        FunctionContract {
            signature: format!("fn {}()", leaf(id)),
            impl_generics: String::new(),
            wasm: true,
            refs: refs.iter().map(|r| r.to_string()).collect(),
        },
    )
}

fn field(id: &str, ty: &str, refs: &[&str]) -> Field {
    Field::new(
        id,
        "",
        FieldContract {
            ty: ty.into(),
            refs: refs.iter().map(|r| r.to_string()).collect(),
        },
    )
}

fn record_type(id: &str, fields: Vec<Field>) -> Type {
    Type::new(
        id,
        "",
        None,
        TypeContract {
            kind: TypeKind::Struct,
            generics: String::new(),
            derives: vec![],
            impls: vec![],
            wire: false,
            wire_form: None,
            constructible: false,
            non_exhaustive: false,
            target: None,
            members: vec![],
            refs: vec![],
        },
        fields,
        vec![],
    )
}

fn sample() -> Surface {
    Surface::new(
        vec![],
        vec![],
        vec![],
        vec![
            function("fn:world::ground", &["type:Block"]),
            function("fn:spawn", &[]),
        ],
        vec![],
        vec![
            record_type(
                "type:Block",
                vec![field("type:Block.state", "State", &["type:State"])],
            ),
            record_type("type:State", vec![]),
            record_type("type:Unrelated", vec![]),
        ],
        vec![],
        vec![Capability::new(
            "capability:horizon:fluids",
            "",
            CapabilityContract {
                kind: CapabilityKind::Block,
                constant: Some("block::FLUIDS".into()),
                handler: None,
                context: None,
                refs: vec![],
            },
        )],
    )
}

#[test]
fn the_document_round_trips_and_its_hash_covers_every_entry() {
    let surface = sample();
    let text = surface.to_json();
    assert!(text.ends_with("}\n"));
    assert_eq!(Surface::read(text.as_bytes()).unwrap(), surface);
    let mut entries: Vec<_> = surface.entries().map(|(id, _)| id).collect();
    entries.sort_unstable();
    assert_eq!(
        entries,
        [
            "capability:horizon:fluids",
            "fn:spawn",
            "fn:world::ground",
            "type:Block",
            "type:Block.state",
            "type:State",
            "type:Unrelated",
        ]
    );
    let mut changed = surface.clone();
    changed.types[0].fields[0] = field("type:Block.state", "u8", &[]);
    assert_ne!(changed.document_hash(), surface.document_hash());
}

#[test]
fn the_closure_follows_references_through_members() {
    let surface = sample();
    let closure = surface.closure(["fn:world::ground", "fn:not_there"]);
    assert_eq!(
        closure.into_iter().collect::<Vec<_>>(),
        [
            "fn:not_there",
            "fn:world::ground",
            "type:Block",
            "type:State"
        ]
    );
}

#[test]
fn leaves_are_what_a_mod_writes() {
    for (id, leaf_name) in [
        ("hook:player_logged_in", "player_logged_in"),
        ("trait:host::items::ItemsHost", "ItemsHost"),
        ("method:Hooks::tick", "tick"),
        ("handle:entity::EntityRef::position", "position"),
        ("fn:world::ground", "ground"),
        ("macro:horizon_mod", "horizon_mod"),
        ("type:entity::EntityRef", "EntityRef"),
        ("type:BlockClick.hand", "hand"),
        ("type:Phase.Load", "Load"),
        ("part:block_entity", "block_entity"),
        ("capability:horizon:fluids", "horizon:fluids"),
    ] {
        assert_eq!(leaf(id), leaf_name, "{id}");
    }
    let surface = sample();
    let identifiers: Vec<_> = surface.identifiers().collect();
    assert!(identifiers.contains(&("FLUIDS", "capability:horizon:fluids")));
    assert!(identifiers.contains(&("state", "type:Block.state")));
}

#[test]
fn the_reader_refuses_with_a_typed_error() {
    let text = sample().to_json();
    let newer = text.replacen("\"schema\": 1", "\"schema\": 2", 1);
    assert_eq!(Surface::read(newer.as_bytes()), Err(ReadError::Schema(2)));
    assert!(matches!(
        Surface::read(&text.as_bytes()[..text.len() / 2]),
        Err(ReadError::Json(_))
    ));
    assert!(matches!(
        Surface::read(b"\x00\xffnot json"),
        Err(ReadError::Json(_))
    ));
    assert!(matches!(Surface::read(b"{}"), Err(ReadError::Json(_))));
    let wrong_type = text.replacen("\"wasm\": true", "\"wasm\": \"yes\"", 1);
    assert!(matches!(
        Surface::read(wrong_type.as_bytes()),
        Err(ReadError::Json(_))
    ));
    let doubled = text.replacen("\"fn:spawn\"", "\"fn:world::ground\"", 1);
    assert_eq!(
        Surface::read(doubled.as_bytes()),
        Err(ReadError::DuplicateId("fn:world::ground".into()))
    );
}

#[test]
fn unknown_fields_are_ignored() {
    let text = sample().to_json();
    let extended = text
        .replacen("{", "{\n  \"future\": {\"x\": [1, 2]},", 1)
        .replacen(
            "\"signature\": \"fn spawn()\"",
            "\"signature\": \"fn spawn()\", \"later\": 3",
            1,
        );
    assert_ne!(extended, text);
    assert_eq!(Surface::read(extended.as_bytes()).unwrap(), sample());
}

#[test]
fn the_contract_hash_leaves_out_doc_and_info_fields() {
    let contract = FunctionContract {
        signature: "fn f(x: u8)".into(),
        impl_generics: String::new(),
        wasm: false,
        refs: vec![],
    };
    let documented = Function::new("fn:f", "Does f.", contract.clone());
    let bare = Function::new("fn:g", "", contract);
    assert_eq!(documented.hash, bare.hash);
    // Independent of the code: SHA-256 of the compact JSON, first 16 bytes.
    use sha2::{Digest, Sha256};
    let expected: String = Sha256::digest(br#"{"signature":"fn f(x: u8)","wasm":false}"#)
        [..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(bare.hash, expected);
}

#[test]
fn deeply_nested_json_neither_overflows_nor_passes_as_records() {
    let deep = |field: &str| {
        let mut text = format!("{{\"schema\": 1, \"hash\": \"\", \"{field}\": ");
        text.push_str(&"[".repeat(100_000));
        text.push_str(&"]".repeat(100_000));
        text.push('}');
        text
    };
    // In a record list: refused, without exhausting the stack.
    assert!(matches!(
        Surface::read(deep("hooks").as_bytes()),
        Err(ReadError::Json(_))
    ));
    // In a field this reader does not know: skipped.
    assert!(Surface::read(deep("future").as_bytes()).is_ok());
}
