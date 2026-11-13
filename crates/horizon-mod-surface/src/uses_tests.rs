use crate::uses::*;
use crate::*;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn method(id: &str, signature: &str, refs: &[&str]) -> Method {
    Method::new(
        id,
        "",
        MethodContract {
            signature: signature.into(),
            required: false,
            wasm: true,
            refs: strings(refs),
        },
    )
}

fn handle(id: &str, signature: &str, refs: &[&str]) -> Function {
    Function::new(
        id,
        "",
        FunctionContract {
            signature: signature.into(),
            impl_generics: String::new(),
            wasm: true,
            refs: strings(refs),
        },
    )
}

fn api_trait(id: &str, role: Role, bounds: &[&str], methods: Vec<Method>) -> Trait {
    Trait::new(
        id,
        "",
        TraitContract {
            role,
            side: Side::Both,
            generics: String::new(),
            bounds: bounds.iter().map(|b| leaf(b).to_owned()).collect(),
            required: vec![],
            refs: strings(bounds),
        },
        methods,
        vec![],
    )
}

fn field(id: &str, ty: &str, refs: &[&str]) -> Field {
    Field::new(
        id,
        "",
        FieldContract {
            ty: ty.into(),
            refs: strings(refs),
        },
    )
}

/// A struct mods can build when `constructible`.
fn api_type(id: &str, generics: &str, constructible: bool, fields: Vec<Field>) -> Type {
    Type::new(
        id,
        "",
        None,
        TypeContract {
            kind: TypeKind::Struct,
            generics: generics.into(),
            wire: false,
            wire_form: None,
            constructible,
            non_exhaustive: false,
            target: None,
            members: vec![],
            refs: vec![],
        },
        fields,
        vec![],
    )
}

/// An exhaustive enum with unit variants.
fn api_enum(id: &str, variants: &[&str]) -> Type {
    Type::new(
        id,
        "",
        None,
        TypeContract {
            kind: TypeKind::Enum,
            generics: String::new(),
            wire: false,
            wire_form: None,
            constructible: true,
            non_exhaustive: false,
            target: None,
            members: variants.iter().map(|v| v.to_string()).collect(),
            refs: vec![],
        },
        vec![],
        variants
            .iter()
            .map(|v| {
                Variant::new(
                    format!("{id}.{v}"),
                    "",
                    VariantContract {
                        fields: vec![],
                        named: false,
                        refs: vec![id.to_owned()],
                    },
                )
            })
            .collect(),
    )
}

fn hook(name: &str, argument_type: &str, refs: &[&str]) -> Hook {
    Hook::new(
        format!("hook:{name}"),
        "",
        HookEngine::default(),
        Counterparts::default(),
        HookContract {
            version: 1,
            side: Side::Server,
            contract: HookKind::Listen,
            scope: None,
            cancellable: None,
            context: None,
            host: None,
            owner: None,
            answered_by_mod: false,
            arguments: vec![Argument {
                name: "player".into(),
                ty: argument_type.into(),
            }],
            output: None,
            refs: strings(refs),
        },
    )
}

/// The pieces of a hand-written surface, changed per test before [`Pieces::build`].
struct Pieces {
    hooks: Vec<Hook>,
    traits: Vec<Trait>,
    handles: Vec<Function>,
    types: Vec<Type>,
    capabilities: Vec<Capability>,
}

impl Pieces {
    fn build(self) -> Surface {
        Surface::new(
            self.hooks,
            self.traits,
            self.handles,
            vec![],
            vec![],
            self.types,
            vec![],
            self.capabilities,
        )
    }

    fn replace_type(&mut self, new: Type) {
        let slot = self.types.iter_mut().find(|t| t.id == new.id).unwrap();
        *slot = new;
    }
}

fn event_type() -> Type {
    api_type(
        "type:PlayerLoggedInEvent",
        "",
        true,
        vec![field(
            "type:PlayerLoggedInEvent.player",
            "PlayerRef",
            &["type:PlayerLoggedInEvent", "type:PlayerRef"],
        )],
    )
}

fn host_trait(extra: Vec<Method>) -> Trait {
    let mut methods = vec![method(
        "method:Host::level_name",
        "fn level_name(&self) -> String",
        &["trait:Host"],
    )];
    methods.extend(extra);
    api_trait("trait:Host", Role::Call, &[], methods)
}

fn base() -> Pieces {
    Pieces {
        hooks: vec![
            hook(
                "player_logged_in",
                "PlayerRef",
                &["type:PlayerLoggedInEvent"],
            ),
            hook("block_broken", "PlayerRef", &[]),
        ],
        traits: vec![
            host_trait(vec![]),
            api_trait(
                "trait:ServerHost",
                Role::Call,
                &["trait:Host"],
                vec![method(
                    "method:ServerHost::give_item",
                    "fn give_item(&mut self, _: PlayerRef, _: ItemStack) -> bool",
                    &["trait:ServerHost", "type:PlayerRef", "type:ItemStack"],
                )],
            ),
            api_trait(
                "trait:Hooks",
                Role::Implement,
                &[],
                vec![method(
                    "method:Hooks::player_logged_in",
                    "fn player_logged_in(&self, _: &mut dyn ServerHost, _: &mut PlayerLoggedInEvent)",
                    &[
                        "trait:Hooks",
                        "trait:ServerHost",
                        "type:PlayerLoggedInEvent",
                    ],
                )],
            ),
            api_trait(
                "trait:Particles",
                Role::Call,
                &[],
                vec![method(
                    "method:Particles::tell",
                    "fn tell(&self, _: u32)",
                    &["trait:Particles"],
                )],
            ),
        ],
        handles: vec![
            handle(
                "handle:PlayerRef::tell",
                "fn tell(self, _: &mut dyn ServerHost, _: Text) -> bool",
                &["type:PlayerRef", "trait:ServerHost", "type:Text"],
            ),
            handle(
                "handle:PlayerRef::inventory",
                "fn inventory(self) -> Inventory",
                &["type:PlayerRef", "type:Inventory"],
            ),
        ],
        types: vec![
            event_type(),
            api_type("type:PlayerRef", "", false, vec![]),
            api_type(
                "type:ItemStack",
                "",
                true,
                vec![field("type:ItemStack.count", "u8", &["type:ItemStack"])],
            ),
            api_type(
                "type:Text",
                "",
                false,
                vec![field("type:Text.content", "String", &["type:Text"])],
            ),
            api_type("type:Inventory", "", false, vec![]),
            api_type("type:Unrelated", "", false, vec![]),
        ],
        capabilities: vec![Capability::new(
            "capability:registries",
            "",
            CapabilityContract {
                kind: CapabilityKind::Manifest,
                constant: None,
                handler: None,
                context: None,
                refs: vec![],
            },
        )],
    }
}

/// A tuple struct mods can build.
fn api_tuple(id: &str, types: &[&str]) -> Type {
    Type::new(
        id,
        "",
        None,
        TypeContract {
            kind: TypeKind::TupleStruct,
            generics: String::new(),
            wire: false,
            wire_form: None,
            constructible: true,
            non_exhaustive: false,
            target: None,
            members: vec![],
            refs: vec![],
        },
        types
            .iter()
            .enumerate()
            .map(|(i, ty)| field(&format!("{id}.{i}"), ty, &[id]))
            .collect(),
        vec![],
    )
}

fn settings(fields: &[(&str, &str)], impls: &[&str]) -> Type {
    let mut record = api_type(
        "type:Settings",
        "",
        true,
        fields
            .iter()
            .map(|(name, ty)| field(&format!("type:Settings.{name}"), ty, &["type:Settings"]))
            .collect(),
    );
    record.impls = impls.iter().map(|i| i.to_string()).collect();
    record
}

/// The base surface with the types the building and path tests use.
fn wider() -> Pieces {
    let mut pieces = base();
    pieces
        .types
        .push(settings(&[("height", "i32")], &["Clone", "Default"]));
    pieces.types.push(api_tuple("type:Pos", &["i32", "i32"]));
    pieces.types.push(api_type("type:Item", "", false, vec![]));
    pieces
        .types
        .push(api_enum("type:TextHover", &["Item", "Plain"]));
    pieces
}

const MOD: &str = r#"use mod_api::{Hooks, ItemStack, PlayerLoggedInEvent, ServerHost};

struct Mod;

impl Hooks for Mod {
    // block_broken is for later; Unrelated too.
    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent) {
        let stack = ItemStack { count: 1 };
        let _ = event.player.inventory();
        host.give_item(event.player, stack);
        let _ = "Particles and block_broken in a string";
    }
}
"#;

fn manifest(names: &[&str]) -> ManifestFacts {
    ManifestFacts {
        names: names.iter().map(|n| n.to_string()).collect(),
        hook_versions: Default::default(),
    }
}

fn scan_on(pieces: Pieces, source: &str) -> Uses {
    uses(
        [("src/lib.rs", source)],
        &manifest(&["registries"]),
        &pieces.build(),
    )
}

fn scan(source: &str) -> Uses {
    scan_on(base(), source)
}

fn ids(uses: &Uses) -> Vec<&str> {
    uses.entries().map(|e| e.id.as_str()).collect()
}

fn entry<'a>(uses: &'a Uses, id: &str) -> &'a Entry {
    uses.entries().find(|e| e.id == id).unwrap()
}

fn verdict(target: Pieces) -> Fit {
    fits(scan(MOD).entries(), &target.build())
}

fn misfit(id: &str, change: Change) -> Misfit {
    Misfit {
        id: id.into(),
        change,
    }
}

fn words(fit: Fit) -> Vec<String> {
    match fit {
        Fit::Fits => vec![],
        Fit::DoesNotFit(misfits) => misfits.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn the_list_is_what_the_source_and_manifest_name_and_what_their_signatures_reach() {
    let uses = scan(MOD);
    assert_eq!(
        ids(&uses),
        [
            "capability:registries",
            "handle:PlayerRef::inventory",
            "hook:player_logged_in",
            "hook:player_logged_in.player",
            "method:Hooks::player_logged_in",
            "method:ServerHost::give_item",
            "trait:Hooks",
            "trait:ServerHost",
            "type:Inventory",
            "type:ItemStack",
            "type:ItemStack.count",
            "type:PlayerLoggedInEvent",
            "type:PlayerLoggedInEvent.player",
            "type:PlayerRef",
        ]
    );
    let origin = |id: &str| {
        uses.items
            .iter()
            .find(|u| u.entry.id == id)
            .unwrap()
            .origin
            .clone()
    };
    let at = |line| Origin::Source {
        file: "src/lib.rs".into(),
        line,
    };
    assert_eq!(origin("trait:Hooks"), at(1));
    assert_eq!(origin("method:ServerHost::give_item"), at(10));
    assert_eq!(origin("type:PlayerLoggedInEvent.player"), at(9));
    assert_eq!(origin("hook:player_logged_in.player"), at(9));
    assert_eq!(origin("capability:registries"), Origin::Manifest);
    assert_eq!(
        origin("type:Inventory"),
        Origin::Via("handle:PlayerRef::inventory".into())
    );
    let base = base().build();
    let hashes: std::collections::BTreeMap<&str, &str> = base.entries().collect();
    for entry in uses.entries() {
        assert_eq!(entry.hash, hashes[entry.id.as_str()], "{}", entry.id);
    }
    // How each named struct is built: `ItemStack` with a full literal on line 8; the event and
    // `PlayerRef` not at all.
    let stack = entry(&uses, "type:ItemStack");
    assert_eq!(stack.built, Some(Built::Full));
    assert_eq!(stack.built_at.as_deref(), Some("src/lib.rs:8"));
    let stack_record = base
        .types
        .iter()
        .find(|t| t.id == "type:ItemStack")
        .unwrap();
    assert_eq!(stack.fields_hash, stack_record.fields_hash);
    assert_eq!(
        entry(&uses, "type:PlayerLoggedInEvent").built,
        Some(Built::None)
    );
    assert_eq!(entry(&uses, "type:PlayerLoggedInEvent").fields_hash, None);
    assert_eq!(entry(&uses, "trait:Hooks").built, None);
}

#[test]
fn the_first_place_is_the_first_file_then_line_whatever_the_order_given() {
    let sources = [("src/b.rs", "\n\nInventory"), ("src/a.rs", "\nInventory")];
    let uses = uses(sources, &ManifestFacts::default(), &base().build());
    assert_eq!(
        uses.items[0].origin,
        Origin::Source {
            file: "src/a.rs".into(),
            line: 2
        }
    );
}

#[test]
fn it_fits_the_same_build() {
    assert_eq!(verdict(base()), Fit::Fits);
}

#[test]
fn it_fits_when_host_gains_a_method_a_hook_is_added_and_a_read_only_type_gains_a_field() {
    let mut target = base();
    target.traits[0] = host_trait(vec![method(
        "method:Host::time",
        "fn time(&self) -> u64",
        &["trait:Host"],
    )]);
    target
        .hooks
        .push(hook("player_left", "PlayerRef", &["type:PlayerRef"]));
    target.replace_type(api_type(
        "type:PlayerRef",
        "",
        false,
        vec![field("type:PlayerRef.uuid", "u128", &["type:PlayerRef"])],
    ));
    // The event the mod reads (and never builds) gains a field too.
    target.replace_type(api_type(
        "type:PlayerLoggedInEvent",
        "",
        true,
        vec![
            field(
                "type:PlayerLoggedInEvent.player",
                "PlayerRef",
                &["type:PlayerLoggedInEvent", "type:PlayerRef"],
            ),
            field(
                "type:PlayerLoggedInEvent.name",
                "String",
                &["type:PlayerLoggedInEvent"],
            ),
        ],
    ));
    assert_eq!(verdict(target), Fit::Fits);
}

#[test]
fn it_fits_when_an_item_it_does_not_use_changes_or_goes() {
    let mut target = base();
    target.hooks.retain(|h| h.id != "hook:block_broken");
    target.replace_type(api_type("type:Unrelated", "<T>", false, vec![]));
    assert_eq!(verdict(target), Fit::Fits);
}

#[test]
fn a_hook_fits_when_it_gains_an_argument_and_not_when_a_named_one_is_retyped() {
    let event = |extra: bool| {
        let mut list = vec![Argument {
            name: "player".into(),
            ty: "PlayerRef".into(),
        }];
        if extra {
            list.push(Argument {
                name: "name".into(),
                ty: "String".into(),
            });
        }
        list
    };
    let mut gained = base();
    gained.hooks[0].contract.arguments = event(true);
    gained.hooks[0] = Hook::new(
        "hook:player_logged_in",
        "",
        HookEngine::default(),
        Counterparts::default(),
        gained.hooks[0].contract.clone(),
    );
    assert_eq!(verdict(gained), Fit::Fits);
    let mut retyped = base();
    retyped.hooks[0] = hook(
        "player_logged_in",
        "ServerPlayerRef",
        &["type:PlayerLoggedInEvent"],
    );
    assert_eq!(
        verdict(retyped),
        Fit::DoesNotFit(vec![misfit(
            "hook:player_logged_in.player",
            Change::Changed
        )])
    );
    // A mod that subscribes but never names the argument fits the retyped hook.
    let quiet = scan("impl Hooks for M { fn player_logged_in(&self) {} }");
    let mut target = base();
    target.hooks[0] = hook(
        "player_logged_in",
        "ServerPlayerRef",
        &["type:PlayerLoggedInEvent"],
    );
    assert_eq!(fits(quiet.entries(), &target.build()), Fit::Fits);
}

#[test]
fn a_hook_version_counts_only_when_the_manifest_names_one() {
    let mut target = base();
    target.hooks[0].contract.version = 2;
    let version_two = target.build();
    let source = [("src/lib.rs", MOD)];
    let plain = uses(source, &manifest(&["registries"]), &base().build());
    assert_eq!(fits(plain.entries(), &version_two), Fit::Fits);
    let mut facts = manifest(&["registries"]);
    facts.hook_versions.insert("player_logged_in".into(), 1);
    let pinned = uses(source, &facts, &base().build());
    assert_eq!(entry(&pinned, "hook:player_logged_in").version, Some(1));
    assert_eq!(
        words(fits(pinned.entries(), &version_two)),
        ["the hook `player_logged_in` is now version 2; this mod names version 1"]
    );
}

#[test]
fn a_removed_used_method_is_named() {
    let mut target = base();
    target.traits[1].methods.clear();
    let Fit::DoesNotFit(misfits) = verdict(target) else {
        panic!("fits")
    };
    assert_eq!(
        misfits,
        [misfit("method:ServerHost::give_item", Change::Removed)]
    );
    assert_eq!(
        misfits[0].to_string(),
        "the method `ServerHost::give_item` (removed)"
    );
}

#[test]
fn a_struct_built_field_by_field_that_gains_a_field_is_named_with_the_line() {
    let mut target = base();
    target.replace_type(api_type(
        "type:ItemStack",
        "",
        true,
        vec![
            field("type:ItemStack.count", "u8", &["type:ItemStack"]),
            field("type:ItemStack.damage", "u16", &["type:ItemStack"]),
        ],
    ));
    assert_eq!(
        words(verdict(target)),
        [
            "the type `ItemStack` gained the field `damage`, and `src/lib.rs:8` builds it field by field"
        ]
    );
}

#[test]
fn a_struct_built_with_a_base_or_not_built_fits_a_new_field() {
    let source = "fn f() -> Settings { let s = Settings { height: 1, ..Default::default() }; s }";
    let uses = scan_on(wider(), source);
    let record = entry(&uses, "type:Settings");
    assert_eq!(record.built, Some(Built::Base));
    assert_eq!(record.built_at.as_deref(), Some("src/lib.rs:1"));
    assert_eq!(record.fields_hash, None);
    let mut target = wider();
    target.replace_type(settings(
        &[("height", "i32"), ("sky", "bool")],
        &["Clone", "Default"],
    ));
    assert_eq!(fits(uses.entries(), &target.build()), Fit::Fits);
    // A named field that is retyped still refuses.
    let mut retyped = wider();
    retyped.replace_type(settings(&[("height", "u32")], &["Clone", "Default"]));
    assert_eq!(
        words(fits(uses.entries(), &retyped.build())),
        ["the field `Settings.height` (changed)"]
    );
}

#[test]
fn how_a_struct_is_built_is_read_from_literals_and_patterns() {
    let built = |source: &str| entry(&scan_on(wider(), source), "type:Settings").built;
    // Literals and patterns with no `..` are full; with a base or rest, base.
    assert_eq!(built("let s = Settings { height: 1 };"), Some(Built::Full));
    assert_eq!(built("let Settings { height } = s;"), Some(Built::Full));
    assert_eq!(built("let Settings { .. } = s;"), Some(Built::Base));
    assert_eq!(built("let s = Settings { ..d };"), Some(Built::Base));
    assert_eq!(
        built("let s = Settings { height: f(Settings { ..d }) };"),
        Some(Built::Full)
    );
    // A range in a field value is not a base.
    assert_eq!(
        built("let s = Settings { height: 0..5 };"),
        Some(Built::Full)
    );
    // Where a type stands, a brace opens a body, not a literal.
    assert_eq!(built("fn f() -> Settings { make() }"), Some(Built::None));
    assert_eq!(
        built("impl Mine for mod_api::Settings { fn x() {} }"),
        Some(Built::None)
    );
    assert_eq!(built("let s: Settings = make();"), Some(Built::None));
    // One full literal anywhere makes it full; unsure (unclosed) is full.
    assert_eq!(
        built("let a = Settings { ..d };\nlet b = mod_api::Settings { height: 2 };"),
        Some(Built::Full)
    );
    assert_eq!(built("let s = Settings { height: 1"), Some(Built::Full));
    // A tuple struct's constructor is a full literal.
    let pos = |source: &str| entry(&scan_on(wider(), source), "type:Pos").built;
    assert_eq!(pos("let p = Pos(1, 2);"), Some(Built::Full));
    assert_eq!(pos("let Pos(x, ..) = p;"), Some(Built::Base));
    assert_eq!(pos("fn f(p: Pos) {}"), Some(Built::None));
}

#[test]
fn a_type_that_gains_an_impl_fits_and_one_that_loses_one_is_named() {
    let uses = scan_on(wider(), "fn f(s: &Settings) {}");
    assert_eq!(entry(&uses, "type:Settings").impls, ["Clone", "Default"]);
    let mut gained = wider();
    gained.replace_type(settings(
        &[("height", "i32")],
        &["Clone", "Default", "Hash"],
    ));
    assert_eq!(fits(uses.entries(), &gained.build()), Fit::Fits);
    let mut lost = wider();
    lost.replace_type(settings(&[("height", "i32")], &["Default"]));
    assert_eq!(
        words(fits(uses.entries(), &lost.build())),
        ["the type `Settings` no longer implements `Clone`"]
    );
}

#[test]
fn a_name_after_a_path_to_its_owner_is_that_member_not_a_type_of_the_same_name() {
    let uses = scan_on(
        wider(),
        "match hover { TextHover::Item { .. } => {}, _ => {} }",
    );
    assert!(ids(&uses).contains(&"type:TextHover.Item"));
    assert!(!ids(&uses).contains(&"type:Item"));
    // Named on its own, it is the type (and the variant, which its owner does not reach).
    let alone = scan_on(wider(), "fn f(i: Item) {}");
    assert!(ids(&alone).contains(&"type:Item"));
    assert!(!ids(&alone).contains(&"type:TextHover.Item"));
}

#[test]
fn a_type_reached_only_through_a_used_signature_changing_is_named() {
    let mut target = base();
    target.replace_type(api_type("type:Inventory", "<'a>", false, vec![]));
    assert_eq!(
        verdict(target),
        Fit::DoesNotFit(vec![misfit("type:Inventory", Change::Changed)])
    );
}

#[test]
fn several_misfits_come_sorted_and_worded() {
    let mut target = base();
    target.hooks[0] = hook("player_logged_in", "u128", &["type:PlayerLoggedInEvent"]);
    target.types.retain(|t| t.id != "type:Inventory");
    target.replace_type(api_type(
        "type:PlayerLoggedInEvent",
        "",
        true,
        vec![field(
            "type:PlayerLoggedInEvent.player",
            "u128",
            &["type:PlayerLoggedInEvent"],
        )],
    ));
    assert_eq!(
        words(verdict(target)),
        [
            "the argument `player_logged_in.player` (changed)",
            "the type `Inventory` (removed)",
            "the field `PlayerLoggedInEvent.player` (changed)",
        ]
    );
}

#[test]
fn a_record_reached_through_others_names_the_counted_record_it_came_from() {
    // The hook references its event type, whose field references `PlayerRef`.
    let uses = scan("fn f() { on(player_logged_in) }");
    let player = uses
        .items
        .iter()
        .find(|u| u.entry.id == "type:PlayerRef")
        .unwrap();
    assert_eq!(player.origin, Origin::Via("hook:player_logged_in".into()));
}

#[test]
fn changes_are_worded_for_people() {
    let say = |id: &str, change: Change| {
        Misfit {
            id: id.into(),
            change,
        }
        .to_string()
    };
    assert_eq!(
        say("type:Kind.Stone", Change::Removed),
        "the variant `Kind::Stone` (removed)"
    );
    assert_eq!(
        say("type:Text.content", Change::Removed),
        "the field `Text.content` (removed)"
    );
    assert_eq!(
        say("type:Pair.0", Change::Removed),
        "the field `Pair.0` (removed)"
    );
    assert_eq!(
        say("handle:PlayerRef::tell", Change::Removed),
        "the method `PlayerRef::tell` (removed)"
    );
    assert_eq!(
        say("capability:horizon:fluids", Change::Removed),
        "the capability `horizon:fluids` (removed)"
    );
    assert_eq!(
        say(
            "type:Settings",
            Change::Fields {
                gained: vec!["sky".into(), "fog".into()],
                at: "src/world.rs:12".into()
            }
        ),
        "the type `Settings` gained the fields `sky`, `fog`, and `src/world.rs:12` builds it field by field"
    );
    assert_eq!(
        say(
            "type:Pos",
            Change::Fields {
                gained: vec![],
                at: "src/a.rs:3".into()
            }
        ),
        "the fields of the type `Pos` changed, and `src/a.rs:3` builds it field by field"
    );
}

#[test]
fn names_in_comments_and_strings_do_not_count() {
    let uses = scan(MOD);
    for id in ["hook:block_broken", "type:Unrelated", "trait:Particles"] {
        assert!(!ids(&uses).contains(&id), "{id}");
    }
}

#[test]
fn a_name_used_only_in_a_macro_invocation_counts() {
    let uses = scan("fn f() { log!(\"{}\", Unrelated::default()); }");
    assert_eq!(ids(&uses), ["capability:registries", "type:Unrelated"]);
}

#[test]
fn a_same_named_method_of_an_unrelated_trait_is_not_pulled_in() {
    // `tell` is a method of `PlayerRef` (named) and of `Particles` (named nowhere).
    let source =
        format!("{MOD}\nfn greet(p: PlayerRef, h: &mut dyn ServerHost) {{ p.tell(h, t); }}");
    let uses = scan(&source);
    assert!(!ids(&uses).contains(&"method:Particles::tell"));
    assert!(!ids(&uses).contains(&"trait:Particles"));
    assert!(ids(&uses).contains(&"handle:PlayerRef::tell"));
    assert!(ids(&uses).contains(&"type:Text"));
}

#[test]
fn a_member_counts_when_its_owner_is_reached_and_not_otherwise() {
    // `inventory` is a method of `PlayerRef`, which the source never names: it is reached
    // through the hook method's event type's field.
    let uses = scan(MOD);
    assert!(ids(&uses).contains(&"handle:PlayerRef::inventory"));
    assert!(ids(&uses).contains(&"type:Inventory"));
    // A member whose owner nothing reaches is dropped.
    let alone = scan("fn f(x: X) { x.inventory() }");
    assert_eq!(ids(&alone), ["capability:registries"]);
}

#[test]
fn the_list_does_not_depend_on_the_order_of_sources() {
    let a = ("src/a.rs", "use mod_api::ServerHost;");
    let b = ("src/b.rs", "fn f(e: PlayerLoggedInEvent) {}");
    let surface = base().build();
    let one = uses([a, b], &manifest(&["registries"]), &surface);
    let two = uses([b, a], &manifest(&["registries"]), &surface);
    assert_eq!(one, two);
    assert_eq!(one.table("abc").to_json(), two.table("abc").to_json());
}

#[test]
fn a_table_reads_back_and_judges_like_the_list() {
    let uses = scan(MOD);
    let table = UsesTable::read(uses.table("afe471c").to_json().as_bytes()).unwrap();
    assert_eq!(table.made_for, "afe471c");
    assert_eq!(table.entries, uses.entries().cloned().collect::<Vec<_>>());
    let mut target = base();
    target.traits[1].methods.clear();
    assert_eq!(
        fits(&table.entries, &target.build()),
        Fit::DoesNotFit(vec![misfit(
            "method:ServerHost::give_item",
            Change::Removed
        )])
    );
}

#[test]
fn a_table_ignores_unknown_fields() {
    let text = r#"{"schema":1,"made_for":"x","later":true,"entries":[{"id":"b","hash":"2","note":0},{"id":"a","hash":"1","built":"base"}]}"#;
    let table = UsesTable::read(text.as_bytes()).unwrap();
    let ids: Vec<&str> = table.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(table.entries[0].built, Some(Built::Base));
}

#[test]
fn bad_tables_are_refused_by_kind() {
    let refused = |text: &[u8]| UsesTable::read(text).unwrap_err();
    assert_eq!(
        refused(br#"{"schema":2,"made_for":"x","entries":[]}"#),
        UsesError::Schema(2)
    );
    assert!(matches!(refused(b"not json"), UsesError::Json(_)));
    assert!(matches!(
        refused(b"{\"schema\":1,\"made_for\""),
        UsesError::Json(_)
    ));
    assert!(matches!(
        refused(br#"{"schema":1,"entries":[]}"#),
        UsesError::Json(_)
    ));
    assert!(matches!(
        refused(br#"{"schema":1,"made_for":"x","entries":[{"id":"a"}]}"#),
        UsesError::Json(_)
    ));
    assert!(matches!(
        refused(br#"{"schema":1,"made_for":"x","entries":[{"id":"a","hash":"1","built":"half"}]}"#),
        UsesError::Json(_)
    ));
    assert_eq!(
        refused(br#"{"schema":1,"made_for":"x","entries":[{"id":"a","hash":"1"},{"id":"a","hash":"2"}]}"#),
        UsesError::DuplicateId("a".into())
    );
    let mut big = br#"{"schema":1,"made_for":"x","entries":[]}"#.to_vec();
    big.resize(MAX_USES_BYTES + 1, b' ');
    assert_eq!(refused(&big), UsesError::TooLarge(MAX_USES_BYTES + 1));
    let mut edge = br#"{"schema":1,"made_for":"x","entries":[]}"#.to_vec();
    edge.resize(MAX_USES_BYTES, b' ');
    assert!(UsesTable::read(&edge).is_ok());
}

#[test]
fn check_says_how_each_struct_is_built() {
    let note = |source: &str, id: &str| entry(&scan_on(wider(), source), id).building_note();
    assert_eq!(
        note(
            "let s = Settings { height: 1, ..Default::default() };",
            "type:Settings"
        )
        .as_deref(),
        Some(
            "`Settings`: built with `..` (src/lib.rs:1), so a new field in a later game build will not break this mod."
        )
    );
    assert_eq!(
        note("\nlet s = Settings { height: 1 };", "type:Settings").as_deref(),
        Some(
            "`Settings`: built field by field (src/lib.rs:2). A new field in a later game build will break this line; end the literal with `..Default::default()` to stay compatible."
        )
    );
    // No `Default` impl: no advice.
    assert_eq!(
        note("let p = Pos(1, 2);", "type:Pos").as_deref(),
        Some(
            "`Pos`: built field by field (src/lib.rs:1). A new field in a later game build will break this line."
        )
    );
    assert_eq!(note("fn f(s: Settings) {}", "type:Settings"), None);
}
