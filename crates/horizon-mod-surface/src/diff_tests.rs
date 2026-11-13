//! One case per classification rule: a hand-written pair of small surfaces, with the class and
//! the words written out here, not derived from the code under test.
use super::*;
use crate::*;

fn hook(name: &str, side: Side, arguments: &[(&str, &str)]) -> Hook {
    Hook::new(
        format!("hook:{name}"),
        "",
        HookEngine::default(),
        Counterparts::default(),
        HookContract {
            version: 1,
            side,
            contract: HookKind::Listen,
            scope: None,
            cancellable: None,
            context: None,
            host: None,
            owner: None,
            answered_by_mod: false,
            arguments: arguments
                .iter()
                .map(|(name, ty)| Argument {
                    name: (*name).into(),
                    ty: (*ty).into(),
                })
                .collect(),
            output: None,
            refs: vec![],
        },
    )
}

fn function(path: &str, signature: &str, wasm: bool) -> Function {
    Function::new(
        format!("fn:{path}"),
        "",
        FunctionContract {
            signature: signature.into(),
            impl_generics: String::new(),
            wasm,
            refs: vec![],
        },
    )
}

fn method(owner: &str, name: &str, required: bool) -> Method {
    Method::new(
        format!("method:{owner}::{name}"),
        "",
        MethodContract {
            signature: format!("fn {name}(&self)"),
            required,
            wasm: true,
            refs: vec![],
        },
    )
}

/// A trait mods implement, with its required list written by hand.
fn implemented(name: &str, methods: Vec<Method>, required: &[&str]) -> Trait {
    Trait::new(
        format!("trait:{name}"),
        "",
        TraitContract {
            role: Role::Implement,
            side: Side::Both,
            generics: String::new(),
            bounds: vec![],
            required: required
                .iter()
                .map(|m| Required {
                    name: (*m).into(),
                    signature: format!("fn {m}(&self)"),
                })
                .collect(),
            refs: vec![],
        },
        methods,
        vec![],
    )
}

fn field(owner: &str, name: &str, ty: &str) -> Field {
    Field::new(
        format!("type:{owner}.{name}"),
        "",
        FieldContract {
            ty: ty.into(),
            refs: vec![],
        },
    )
}

fn variant(owner: &str, name: &str) -> Variant {
    Variant::new(
        format!("type:{owner}.{name}"),
        "",
        VariantContract {
            fields: vec![],
            named: false,
            refs: vec![],
        },
    )
}

fn contract(kind: TypeKind, constructible: bool, members: &[&str]) -> TypeContract {
    TypeContract {
        kind,
        generics: String::new(),
        derives: vec![],
        impls: vec![],
        wire: false,
        wire_form: None,
        constructible,
        non_exhaustive: kind == TypeKind::Enum && !constructible,
        target: None,
        members: members.iter().map(|m| (*m).into()).collect(),
        refs: vec![],
    }
}

/// A struct; `constructible` lists its fields in the contract as the generator does.
fn structure(name: &str, constructible: bool, fields: &[(&str, &str)]) -> Type {
    let members: Vec<String> = fields.iter().map(|(n, t)| format!("{n}: {t}")).collect();
    let members: Vec<&str> = members.iter().map(String::as_str).collect();
    Type::new(
        format!("type:{name}"),
        "",
        None,
        contract(
            TypeKind::Struct,
            constructible,
            if constructible { &members } else { &[] },
        ),
        fields.iter().map(|(n, t)| field(name, n, t)).collect(),
        vec![],
    )
}

fn enumeration(name: &str, exhaustive: bool, variants: &[&str]) -> Type {
    Type::new(
        format!("type:{name}"),
        "",
        None,
        contract(
            TypeKind::Enum,
            exhaustive,
            if exhaustive { variants } else { &[] },
        ),
        vec![],
        variants.iter().map(|v| variant(name, v)).collect(),
    )
}

#[derive(Default)]
struct Parts {
    hooks: Vec<Hook>,
    traits: Vec<Trait>,
    functions: Vec<Function>,
    types: Vec<Type>,
}

fn surface(parts: Parts) -> Surface {
    Surface::new(
        parts.hooks,
        parts.traits,
        vec![],
        parts.functions,
        vec![],
        parts.types,
        vec![],
        vec![],
    )
}

fn hooks(hooks: Vec<Hook>) -> Surface {
    surface(Parts {
        hooks,
        ..Parts::default()
    })
}

fn functions(functions: Vec<Function>) -> Surface {
    surface(Parts {
        functions,
        ..Parts::default()
    })
}

fn types(types: Vec<Type>) -> Surface {
    surface(Parts {
        types,
        ..Parts::default()
    })
}

fn traits(traits: Vec<Trait>) -> Surface {
    surface(Parts {
        traits,
        ..Parts::default()
    })
}

/// The one change, as (id, kind, class, lines, members).
fn only(diff: &Diff) -> (&str, ChangeKind, Class, Vec<&str>, Vec<&str>) {
    assert_eq!(diff.changes.len(), 1, "{}", diff.to_text());
    let c = &diff.changes[0];
    (
        &c.id,
        c.kind,
        c.class,
        c.what.iter().map(String::as_str).collect(),
        c.members.iter().map(String::as_str).collect(),
    )
}

#[test]
fn identical_surfaces_have_no_changes() {
    let a = hooks(vec![hook("tick", Side::Server, &[("level", "LevelRef")])]);
    let d = diff(&a, &a.clone());
    assert_eq!(d.verdict, Verdict::Identical);
    assert!(d.changes.is_empty());
    assert!(
        d.to_text()
            .starts_with("Surface: identical (0 breaking, 0 additive")
    );
}

#[test]
fn a_removed_record_is_breaking() {
    let old = functions(vec![function("spawn", "fn spawn()", true)]);
    let d = diff(&old, &functions(vec![]));
    assert_eq!(d.verdict, Verdict::Breaking);
    assert_eq!(
        only(&d),
        (
            "fn:spawn",
            ChangeKind::Removed,
            Class::Breaking,
            vec!["removed"],
            vec![]
        )
    );
}

#[test]
fn a_new_record_is_additive() {
    let new = functions(vec![function("spawn", "fn spawn()", true)]);
    let d = diff(&functions(vec![]), &new);
    assert_eq!(d.verdict, Verdict::Additive);
    assert_eq!(
        only(&d),
        (
            "fn:spawn",
            ChangeKind::Added,
            Class::Additive,
            vec!["added"],
            vec![]
        )
    );
}

#[test]
fn a_rename_is_a_removal_and_an_addition() {
    let old = functions(vec![function("spawn", "fn spawn()", true)]);
    let new = functions(vec![function("summon", "fn summon()", true)]);
    let d = diff(&old, &new);
    let seen: Vec<_> = d.changes.iter().map(|c| (c.id.as_str(), c.kind)).collect();
    assert_eq!(
        seen,
        [
            ("fn:spawn", ChangeKind::Removed),
            ("fn:summon", ChangeKind::Added)
        ]
    );
}

#[test]
fn a_retyped_hook_argument_is_breaking_and_named() {
    let old = hooks(vec![hook(
        "player_logged_in",
        Side::Server,
        &[("player", "PlayerRef")],
    )]);
    let new = hooks(vec![hook(
        "player_logged_in",
        Side::Server,
        &[("player", "ServerPlayerRef")],
    )]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "hook:player_logged_in",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["argument `player`: `PlayerRef` → `ServerPlayerRef`"],
            vec![]
        )
    );
}

#[test]
fn a_new_or_removed_hook_argument_is_breaking() {
    let old = hooks(vec![hook("tick", Side::Server, &[("level", "LevelRef")])]);
    let new = hooks(vec![hook(
        "tick",
        Side::Server,
        &[("level", "LevelRef"), ("phase", "u8")],
    )]);
    let d = diff(&old, &new);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (Class::Breaking, vec!["argument `phase: u8` added"])
    );
    let d = diff(&new, &old);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (Class::Breaking, vec!["argument `phase` removed"])
    );
}

#[test]
fn a_hook_side_change_is_breaking() {
    let old = hooks(vec![hook("tick", Side::Server, &[])]);
    let new = hooks(vec![hook("tick", Side::Both, &[])]);
    let d = diff(&old, &new);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (Class::Breaking, vec!["side: server → both"])
    );
}

#[test]
fn leaving_wasm_breaks_and_joining_it_adds() {
    let on = functions(vec![function("ground", "fn ground()", true)]);
    let off = functions(vec![function("ground", "fn ground()", false)]);
    let d = diff(&on, &off);
    let (_, _, class, what, _) = only(&d);
    assert_eq!((class, what), (Class::Breaking, vec!["no longer on wasm"]));
    let d = diff(&off, &on);
    let (_, _, class, what, _) = only(&d);
    assert_eq!((class, what), (Class::Additive, vec!["now on wasm"]));
}

#[test]
fn a_changed_signature_is_breaking_and_shown() {
    let old = functions(vec![function("name", "fn name(x: &'a str)", true)]);
    let new = functions(vec![function("name", "fn name(x: &'b str)", true)]);
    let d = diff(&old, &new);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (
            Class::Breaking,
            vec!["signature: `fn name(x: &'a str)` → `fn name(x: &'b str)`"]
        )
    );
}

#[test]
fn a_field_on_a_constructible_struct_is_one_breaking_change_on_the_container() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i32"), ("y", "i32")])]);
    let d = diff(&old, &new);
    assert_eq!(d.verdict, Verdict::Breaking);
    assert_eq!(
        only(&d),
        (
            "type:Pos",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["field `y: i32` added: a `Pos { .. }` literal in a mod no longer compiles"],
            vec!["type:Pos.y"]
        )
    );
}

#[test]
fn a_retyped_field_of_a_constructible_struct_is_reported_once() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i64")])]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "type:Pos",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["field `x`: `i32` → `i64`"],
            vec!["type:Pos.x"]
        )
    );
}

#[test]
fn a_field_on_a_type_mods_only_read_is_additive() {
    let old = types(vec![structure("Info", false, &[("x", "i32")])]);
    let new = types(vec![structure(
        "Info",
        false,
        &[("x", "i32"), ("y", "i32")],
    )]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "type:Info.y",
            ChangeKind::Added,
            Class::Additive,
            vec!["added"],
            vec![]
        )
    );
    // Its removal is still breaking.
    let d = diff(&new, &old);
    let (id, _, class, ..) = only(&d);
    assert_eq!((id, class), ("type:Info.y", Class::Breaking));
}

#[test]
fn a_variant_on_an_exhaustive_enum_breaks_and_on_a_non_exhaustive_one_adds() {
    let old = types(vec![enumeration("Hand", true, &["Main"])]);
    let new = types(vec![enumeration("Hand", true, &["Main", "Off"])]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "type:Hand",
            ChangeKind::Changed,
            Class::Breaking,
            vec![
                "variant `Off` added: an exhaustive `match` on `Hand` in a mod no longer compiles"
            ],
            vec!["type:Hand.Off"]
        )
    );
    let old = types(vec![enumeration("Hand", false, &["Main"])]);
    let new = types(vec![enumeration("Hand", false, &["Main", "Off"])]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "type:Hand.Off",
            ChangeKind::Added,
            Class::Additive,
            vec!["added"],
            vec![]
        )
    );
}

#[test]
fn a_required_method_on_an_implemented_trait_breaks_and_a_defaulted_one_adds() {
    let old = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", true)],
        &["tick"],
    )]);
    let required = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", true), method("Part", "save", true)],
        &["tick", "save"],
    )]);
    assert_eq!(
        only(&diff(&old, &required)),
        (
            "trait:Part",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["required `fn save(&self)` added: every implementation of `Part` must write it"],
            vec!["method:Part::save"]
        )
    );
    let defaulted = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", true), method("Part", "save", false)],
        &["tick"],
    )]);
    assert_eq!(
        only(&diff(&old, &defaulted)),
        (
            "method:Part::save",
            ChangeKind::Added,
            Class::Additive,
            vec!["added"],
            vec![]
        )
    );
}

#[test]
fn a_method_that_gains_a_default_is_additive() {
    let old = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", true)],
        &["tick"],
    )]);
    let new = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", false)],
        &[],
    )]);
    let d = diff(&old, &new);
    assert_eq!(d.verdict, Verdict::Additive, "{}", d.to_text());
    assert_eq!(
        only(&d),
        (
            "trait:Part",
            ChangeKind::Changed,
            Class::Additive,
            vec!["`tick` is no longer required"],
            vec!["method:Part::tick"]
        )
    );
}

#[test]
fn a_removed_container_carries_its_members() {
    let old = types(vec![structure("Pos", false, &[("x", "i32"), ("y", "i32")])]);
    assert_eq!(
        only(&diff(&old, &types(vec![]))),
        (
            "type:Pos",
            ChangeKind::Removed,
            Class::Breaking,
            vec!["removed"],
            vec!["type:Pos.x", "type:Pos.y"]
        )
    );
}

#[test]
fn a_derive_added_is_additive_and_one_removed_breaks() {
    let plain = types(vec![structure("Id", false, &[])]);
    let mut derived = plain.clone();
    derived.types[0] = Type::new(
        "type:Id",
        "",
        None,
        TypeContract {
            derives: vec!["Clone".into()],
            ..contract(TypeKind::Struct, false, &[])
        },
        vec![],
        vec![],
    );
    let d = diff(&plain, &derived);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (Class::Additive, vec!["derive `Clone` added"])
    );
    let d = diff(&derived, &plain);
    let (_, _, class, what, _) = only(&d);
    assert_eq!(
        (class, what),
        (Class::Breaking, vec!["derive `Clone` removed"])
    );
}

#[test]
fn breaking_changes_come_first_and_notes_declare_them() {
    let old = functions(vec![function("a", "fn a()", true)]);
    let new = functions(vec![function("b", "fn b()", true)]);
    let mut d = diff(&old, &new);
    assert_eq!(d.undeclared().count(), 1);
    let note = Note {
        file: "rename.toml".into(),
        why: "Shorter name.".into(),
        migrate: Some("Call `b`.".into()),
    };
    let stale = d.attach(&note, &["fn:a".into(), "fn:never".into()]);
    assert_eq!(stale, ["fn:never"]);
    assert_eq!(d.undeclared().count(), 0);
    assert_eq!(
        d.to_text(),
        "Surface: breaking (1 breaking, 1 additive; 0 without a note)\n\
         Old OLD, new NEW\n\
         \n\
         Breaking (1)\n\
         - fn:a (removed)\n    why (rename.toml): Shorter name.\n    migrate: Call `b`.\n\
         \n\
         Additive (1)\n\
         - fn:b (added)\n"
            .replace("OLD", &old.hash)
            .replace("NEW", &new.hash)
    );
}

#[test]
fn a_note_may_name_a_folded_member() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i32"), ("y", "i32")])]);
    let mut d = diff(&old, &new);
    let note = Note {
        file: "y.toml".into(),
        why: "Positions are 2D now.".into(),
        migrate: None,
    };
    assert!(d.attach(&note, &["type:Pos.y".into()]).is_empty());
    assert_eq!(d.changes[0].notes, [note]);
}

#[test]
fn the_diff_round_trips_as_json() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i32"), ("y", "i32")])]);
    let d = diff(&old, &new);
    let text = serde_json::to_string(&d).unwrap();
    assert!(text.contains(r#""verdict":"breaking""#), "{text}");
    assert!(text.contains(r#""class":"breaking""#), "{text}");
    assert_eq!(serde_json::from_str::<Diff>(&text).unwrap(), d);
}
