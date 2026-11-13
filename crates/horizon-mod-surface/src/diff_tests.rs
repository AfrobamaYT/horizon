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
        wire: false,
        wire_form: None,
        constructible,
        non_exhaustive: kind == TypeKind::Enum && !constructible,
        target: None,
        members: members.iter().map(|m| (*m).into()).collect(),
        refs: vec![],
    }
}

fn structure(name: &str, constructible: bool, fields: &[(&str, &str)]) -> Type {
    Type::new(
        format!("type:{name}"),
        "",
        None,
        contract(TypeKind::Struct, constructible, &[]),
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
            .starts_with("Surface: identical (0 breaking, 0 conditional, 0 additive")
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
            "hook:player_logged_in.player",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["argument `player`: `PlayerRef` → `ServerPlayerRef`"],
            vec![]
        )
    );
}

#[test]
fn a_new_hook_argument_is_conditional_and_a_removed_one_breaks() {
    let old = hooks(vec![hook("tick", Side::Server, &[("level", "LevelRef")])]);
    let new = hooks(vec![hook(
        "tick",
        Side::Server,
        &[("level", "LevelRef"), ("phase", "u8")],
    )]);
    let d = diff(&old, &new);
    assert_eq!(d.verdict, Verdict::Conditional);
    assert_eq!(d.undeclared().count(), 0);
    assert_eq!(
        only(&d),
        (
            "hook:tick.phase",
            ChangeKind::Added,
            Class::Conditional,
            vec![
                "argument `phase: u8` added to `tick`: breaks only a mod that builds this hook's event field by field, without `..`"
            ],
            vec![]
        )
    );
    assert_eq!(
        only(&diff(&new, &old)),
        (
            "hook:tick.phase",
            ChangeKind::Removed,
            Class::Breaking,
            vec!["removed"],
            vec![]
        )
    );
}

#[test]
fn a_hook_version_is_conditional() {
    let old = hooks(vec![hook("tick", Side::Server, &[])]);
    let mut new = old.clone();
    new.hooks[0].contract.version = 2;
    assert_eq!(new.hooks[0].hash, old.hooks[0].hash);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "hook:tick",
            ChangeKind::Changed,
            Class::Conditional,
            vec![
                "version 1 → 2: affects only a mod that names this hook's version in its manifest"
            ],
            vec![]
        )
    );
}

#[test]
fn a_hook_that_gains_an_output_breaks_and_carries_it() {
    let old = hooks(vec![hook("damage", Side::Server, &[])]);
    let mut contract = old.hooks[0].contract.clone();
    contract.output = Some(Argument {
        name: "amount".into(),
        ty: "f32".into(),
    });
    let new = hooks(vec![Hook::new(
        "hook:damage",
        "",
        HookEngine::default(),
        Counterparts::default(),
        contract,
    )]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "hook:damage",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["now has an output"],
            vec!["hook:damage.amount"]
        )
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
fn a_field_on_a_constructible_struct_is_conditional_and_named() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i32"), ("y", "i32")])]);
    let d = diff(&old, &new);
    assert_eq!(d.verdict, Verdict::Conditional);
    assert_eq!(
        only(&d),
        (
            "type:Pos.y",
            ChangeKind::Added,
            Class::Conditional,
            vec![
                "field `y: i32` added to `Pos`: breaks only a mod that builds `Pos` field by field, without `..`"
            ],
            vec![]
        )
    );
}

#[test]
fn a_retyped_field_is_breaking_and_named() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i64")])]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "type:Pos.x",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["field `x`: `i32` → `i64`"],
            vec![]
        )
    );
}

#[test]
fn a_new_constructible_struct_is_additive_with_its_fields() {
    let new = types(vec![structure("Pos", true, &[("x", "i32")])]);
    assert_eq!(
        only(&diff(&types(vec![]), &new)),
        (
            "type:Pos",
            ChangeKind::Added,
            Class::Additive,
            vec!["added"],
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

fn with_impls(impls: &[&str]) -> Surface {
    let mut surface = types(vec![structure("Id", false, &[])]);
    surface.types[0].impls = impls.iter().map(|i| (*i).into()).collect();
    surface
}

#[test]
fn an_impl_gained_is_additive_and_one_lost_breaks_by_name() {
    let plain = with_impls(&["Clone"]);
    let hashed = with_impls(&["Clone", "Hash"]);
    assert_eq!(hashed.types[0].hash, plain.types[0].hash);
    assert_eq!(
        only(&diff(&plain, &hashed)),
        (
            "type:Id",
            ChangeKind::Changed,
            Class::Additive,
            vec!["now implements `Hash`"],
            vec![]
        )
    );
    assert_eq!(
        only(&diff(&hashed, &plain)),
        (
            "type:Id",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["the type `Id` no longer implements `Hash`"],
            vec![]
        )
    );
    // A derive that becomes a hand-written impl is the same entry: nothing changed.
    assert_eq!(
        diff(&with_impls(&["Default"]), &with_impls(&["Default"])).verdict,
        Verdict::Identical
    );
}

fn wired(wire: bool, form: Option<&str>) -> Surface {
    types(vec![Type::new(
        "type:Id",
        "",
        None,
        TypeContract {
            wire,
            wire_form: form.map(str::to_owned),
            ..contract(TypeKind::Struct, false, &[])
        },
        vec![],
        vec![],
    )])
}

#[test]
fn a_wire_change_adds_when_a_type_starts_crossing_and_is_conditional_otherwise() {
    let none = wired(false, None);
    let derive = wired(true, Some("derive"));
    let via = wired(true, Some("via:u32"));
    let class_and_lines = |a: &Surface, b: &Surface| {
        let d = diff(a, b);
        let (_, _, class, what, _) = only(&d);
        (
            class,
            what.into_iter().map(str::to_owned).collect::<Vec<_>>(),
        )
    };
    assert_eq!(
        class_and_lines(&none, &derive),
        (
            Class::Additive,
            vec!["now crosses the wasm boundary".into()]
        )
    );
    assert_eq!(
        class_and_lines(&derive, &none),
        (
            Class::Conditional,
            vec!["no longer crosses the wasm boundary: affects wasm mods only".into()]
        )
    );
    assert_eq!(
        class_and_lines(&derive, &via),
        (
            Class::Conditional,
            vec!["wire form: `derive` → `via:u32`: affects wasm mods only".into()]
        )
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
        "Surface: breaking (1 breaking, 0 conditional, 1 additive; 0 breaking without a note)\n\
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
    let old = types(vec![enumeration("Hand", true, &["Main"])]);
    let new = types(vec![enumeration("Hand", true, &["Main", "Off"])]);
    let mut d = diff(&old, &new);
    let note = Note {
        file: "off.toml".into(),
        why: "Two hands.".into(),
        migrate: None,
    };
    assert!(d.attach(&note, &["type:Hand.Off".into()]).is_empty());
    assert_eq!(d.changes[0].notes, [note]);
}

#[test]
fn the_diff_round_trips_as_json() {
    let old = types(vec![structure("Pos", true, &[("x", "i32")])]);
    let new = types(vec![structure("Pos", true, &[("x", "i32"), ("y", "i32")])]);
    let d = diff(&old, &new);
    let text = serde_json::to_string(&d).unwrap();
    assert!(text.contains(r#""verdict":"conditional""#), "{text}");
    assert!(text.contains(r#""class":"conditional""#), "{text}");
    assert_eq!(serde_json::from_str::<Diff>(&text).unwrap(), d);
}

#[test]
fn a_folded_member_can_make_its_container_breaking() {
    // The trait's own change only widens it, but a member folded into it was removed.
    let old = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", true), method("Part", "save", false)],
        &["tick"],
    )]);
    let new = traits(vec![implemented(
        "Part",
        vec![method("Part", "tick", false)],
        &[],
    )]);
    assert_eq!(
        only(&diff(&old, &new)),
        (
            "trait:Part",
            ChangeKind::Changed,
            Class::Breaking,
            vec!["`tick` is no longer required"],
            vec!["method:Part::save", "method:Part::tick"]
        )
    );
}
