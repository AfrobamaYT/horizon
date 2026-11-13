//! The export over a small fixture API, and over the real tree.
use super::*;
use std::collections::BTreeMap;

/// The fixture: path below the root, and text.
const FILES: &[(&str, &str)] = &[
    (
        "crates/horizon-mod-api/src/lib.rs",
        "pub use mod_vocab::*;\n",
    ),
    (
        "crates/horizon-mod-vocab/src/lib.rs",
        r#"
pub mod entity;
pub use entity::EntityRef;
pub use horizon_mod_api_macros::{Wire, horizon_mod};
#[doc(hidden)]
pub use events::*;
#[doc(hidden)]
pub mod internal {
    pub struct Secret;
}
mod events {
    use super::*;
    /// The event of `ping`.
    pub struct PingEvent<'a> {
        pub who: EntityRef,
        pub lines: &'a mut Vec<(Text, u32)>,
        pub cancelled: bool,
    }
    /// The event of `tick_block`.
    pub struct TickBlockEvent {
        pub pos: Point,
    }
}
/// A text: parts of parts.
#[derive(Clone, Debug, Wire)]
pub struct Text {
    pub parts: Vec<Text>,
    style: u8,
}
/// A point.
#[derive(Clone, Copy, Wire)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
/// Limits a mod reads but cannot build.
#[non_exhaustive]
pub struct Limits {
    pub max: u32,
}
/// A colour.
#[non_exhaustive]
pub enum Colour {
    Red,
    Rgb(u8, u8, u8),
}
/// A mode.
pub enum Mode {
    On,
    Off { reason: String },
}
/// What a mod answers.
pub trait Hooks {
    /// Pinged.
    fn ping(&self, host: &mut dyn ServerHost, event: &mut PingEvent<'_>) {
        let _ = (host, event);
    }
    /// Must be written.
    fn name(&self) -> &'static str;
}
/// The game.
pub trait Host {
    /// The time.
    fn time(&self) -> u64;
}
/// The server's game.
pub trait ServerHost: Host {
    /// Saves.
    ///
    /// Not on wasm: it writes
    /// files.
    fn save(&mut self, slot: u8);
    /// Sends a text.
    fn send(&mut self, text: Text) -> bool;
}
/// A block's part.
pub trait BlockPart {}
/// Finds the ground.
pub fn ground(at: EntityRef) -> Option<Point> {
    let _ = at;
    None
}
/// The first part.
pub fn first<'a>(text: &'a Text) -> &'a Text {
    text
}
/// A capability key.
pub struct CapabilityKey<T: ?Sized> {
    pub name: &'static str,
    pub handler: std::marker::PhantomData<T>,
}
/// Fluids, through a handler.
pub trait FluidHandler {}
/// Fluids.
pub const FLUIDS: CapabilityKey<dyn FluidHandler> = CapabilityKey {
    name: "fixture:fluids",
    handler: std::marker::PhantomData,
};
/// Fluids, made by a constructor.
pub const MORE_FLUIDS: CapabilityKey<dyn FluidHandler> = CapabilityKey::new("fixture:more_fluids");
pub const LIMIT: u32 = 3;
/// Makes a key.
#[macro_export]
macro_rules! key {
    () => {};
}
/// A result of the fixture.
pub type Outcome<T> = Result<T, Colour>;
"#,
    ),
    (
        "crates/horizon-mod-vocab/src/entity.rs",
        r#"
/// A live entity.
#[derive(Clone, Copy, Wire)]
pub struct EntityRef(u32);
impl EntityRef {
    /// Its id.
    pub fn id(self) -> u32 {
        self.0
    }
    /// Its file.
    ///
    /// Not on wasm: it reads a file.
    pub fn path(self) -> crate::Outcome<String> {
        Ok(String::new())
    }
    fn private(self) {}
}
"#,
    ),
    (
        "crates/horizon-mod-api-macros/src/lib.rs",
        r#"
/// The derive.
#[proc_macro_derive(Wire, attributes(wire))]
pub fn derive_wire(input: TokenStream) -> TokenStream { input }
/// Marks the mod.
#[proc_macro_attribute]
pub fn horizon_mod(_: TokenStream, input: TokenStream) -> TokenStream { input }
"#,
    ),
    (
        "crates/horizon-mod-package/src/manifest.rs",
        r#"
pub enum Capability {
    /// New registries.
    Registries,
}
"#,
    ),
];

const HOOKS: &str = r#"
pub trait Hooks {
    /// Something pinged.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancelled",
        target = "horizon_client::game::Game::ping",
        neoforge = "PingEvent"
    )]
    fn ping();
    /// A block ticks.
    #[hook(
        side = "server",
        contract = "listen",
        scope = "block",
        key = "pos",
        target = "horizon_client::game::GameLevel::tick_block",
        neoforge = "none"
    )]
    fn tick_block();
}
"#;

fn boundary() -> Boundary {
    let mut boundary = Boundary::default();
    boundary.roles.insert("Hooks".into(), Role::Implement);
    boundary.roles.insert("Host".into(), Role::Call);
    boundary.roles.insert("ServerHost".into(), Role::Call);
    boundary.sides.insert("ServerHost".into(), Side::Server);
    boundary.unbound.insert(
        ("ServerHost".into(), "send".into()),
        "Text does not cross".into(),
    );
    boundary
}

/// The export of the fixture with `edits` (file, from, to) applied.
fn export_fixture(edits: &[(&str, &str, &str)], hooks: &str) -> Export {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (path, text) in FILES {
        let mut text = text.to_string();
        for (file, from, to) in edits {
            if file == path {
                assert!(text.contains(from), "{from} not in {path}");
                text = text.replacen(from, to, 1);
            }
        }
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let catalog = Catalog::parse(&[("hooks/ping.rs".into(), hooks.into())]).unwrap();
    let mut index = Index::new(root, &[]);
    build(root, &mut index, &catalog.hooks, &boundary()).unwrap()
}

const LIB: &str = "crates/horizon-mod-vocab/src/lib.rs";

fn hashes(export: &Export) -> BTreeMap<String, String> {
    export
        .surface
        .entries()
        .map(|(id, hash)| (id.to_owned(), hash.to_owned()))
        .collect()
}

/// The ids whose hash changed, and the ids added, from the base fixture to the edited one.
fn changes(edits: &[(&str, &str, &str)], hooks: &str) -> (Vec<String>, Vec<String>) {
    let before = hashes(&export_fixture(&[], HOOKS));
    let after = hashes(&export_fixture(edits, hooks));
    let changed = before
        .iter()
        .filter(|(id, hash)| after.get(*id).is_some_and(|new| new != *hash))
        .map(|(id, _)| id.clone())
        .collect();
    let added = after
        .keys()
        .filter(|id| !before.contains_key(*id))
        .cloned()
        .collect();
    (changed, added)
}

/// `value` with every `hash` and `shape` written as `*`.
fn masked(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| match key.as_str() {
                    "hash" | "shape" => (key, serde_json::Value::from("*")),
                    _ => (key, masked(value)),
                })
                .collect(),
        ),
        serde_json::Value::Array(list) => {
            serde_json::Value::Array(list.into_iter().map(masked).collect())
        }
        value => value,
    }
}

#[test]
fn the_fixture_exports_the_expected_document() {
    let export = export_fixture(&[], HOOKS);
    let written: serde_json::Value = serde_json::from_str(&export.surface.to_json()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixture_expected.json")).unwrap();
    assert_eq!(masked(written), expected, "{}", export.surface.to_json());
    assert_eq!(
        export.notes,
        ["constant LIMIT: no record kind for constants"]
    );
}

#[test]
fn two_exports_are_byte_identical() {
    let a = export_fixture(&[], HOOKS).surface.to_json();
    let b = export_fixture(&[], HOOKS).surface.to_json();
    assert_eq!(a, b);
    assert!(a.ends_with("}\n"));
}

#[test]
fn each_declaration_aspect_changes_exactly_its_records_hash() {
    let cases: &[(&str, &[(&str, &str, &str)], &str, &[&str])] = &[
        (
            "a method's argument type",
            &[(
                LIB,
                "fn save(&mut self, slot: u8);",
                "fn save(&mut self, slot: u16);",
            )],
            HOOKS,
            &["method:ServerHost::save"],
        ),
        (
            "a lifetime",
            &[(LIB, "-> &'a Text {", "-> &'static Text {")],
            HOOKS,
            &["fn:first"],
        ),
        (
            "a field added to a constructible struct",
            &[(
                LIB,
                "    pub y: i32,\n",
                "    pub y: i32,\n    pub z: i32,\n",
            )],
            HOOKS,
            &["type:Point"],
        ),
        (
            "a variant added to an exhaustive enum",
            &[(LIB, "    On,\n", "    On,\n    Paused,\n")],
            HOOKS,
            &["type:Mode"],
        ),
        (
            "non_exhaustive toggled",
            &[(LIB, "/// A mode.\n", "/// A mode.\n#[non_exhaustive]\n")],
            HOOKS,
            &["type:Mode"],
        ),
        (
            "a derive added",
            &[(LIB, "/// A mode.\n", "/// A mode.\n#[derive(Debug)]\n")],
            HOOKS,
            &["type:Mode"],
        ),
        (
            "a required method added to a trait mods implement",
            &[(
                LIB,
                "    fn name(&self)",
                "    fn more(&self);\n    fn name(&self)",
            )],
            HOOKS,
            &["trait:Hooks"],
        ),
        (
            "a handle method's return type",
            &[(
                "crates/horizon-mod-vocab/src/entity.rs",
                "pub fn id(self) -> u32",
                "pub fn id(self) -> u64",
            )],
            HOOKS,
            &["handle:EntityRef::id"],
        ),
        (
            "a hook's side",
            &[],
            &HOOKS.replacen("side = \"server\"", "side = \"both\"", 1),
            &["hook:ping"],
        ),
        (
            "a hook argument's type",
            &[(
                LIB,
                "pub cancelled: bool,",
                "pub cancelled: bool,\n        pub extra: u8,",
            )],
            HOOKS,
            &["hook:ping", "type:PingEvent"],
        ),
        (
            "a doc line",
            &[(LIB, "/// A point.", "/// A point on a plane.")],
            HOOKS,
            &[],
        ),
        (
            "a hook's doc, engine target and counterpart",
            &[],
            &HOOKS
                .replace("Something pinged.", "Something was pinged.")
                .replace("Game::ping", "Game::pinged")
                .replace("neoforge = \"PingEvent\"", "neoforge = \"PingedEvent\""),
            &[],
        ),
        (
            "a method added to a trait mods call",
            &[(
                LIB,
                "    fn time(&self) -> u64;",
                "    fn time(&self) -> u64;\n    fn day(&self) -> u64;",
            )],
            HOOKS,
            &[],
        ),
        (
            "a defaulted method added to a trait mods implement",
            &[(
                LIB,
                "    fn name(&self)",
                "    fn more(&self) {}\n    fn name(&self)",
            )],
            HOOKS,
            &[],
        ),
        (
            "a field added to a type mods cannot build",
            &[(
                LIB,
                "    style: u8,\n",
                "    style: u8,\n    pub size: u8,\n",
            )],
            HOOKS,
            &[],
        ),
        (
            "a variant added to a non-exhaustive enum",
            &[(LIB, "    Red,\n", "    Red,\n    Blue,\n")],
            HOOKS,
            &[],
        ),
    ];
    for (aspect, edits, hooks, expected) in cases {
        let (changed, _) = changes(edits, hooks);
        assert_eq!(changed, *expected, "{aspect}");
    }
    // A new hook (with its event type) adds records and changes none.
    let pong = HOOKS.replace(
        "fn tick_block();\n}",
        "fn tick_block();\n    /// Ponged.\n    #[hook(side = \"client\", contract = \"listen\", target = \"x::pong\", neoforge = \"none\")]\n    fn pong();\n}",
    );
    let (changed, added) = changes(
        &[(
            LIB,
            "mod events {\n    use super::*;",
            "mod events {\n    use super::*;\n    /// The event of `pong`.\n    pub struct PongEvent { pub n: u8 }",
        )],
        &pong,
    );
    assert_eq!(changed, Vec::<String>::new());
    assert!(added.contains(&"hook:pong".to_owned()), "{added:?}");
}

#[test]
fn shapes_follow_the_wire_layout() {
    let base = export_fixture(&[], HOOKS);
    let shape = |export: &Export, id: &str| {
        export
            .surface
            .types
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .shape
            .clone()
    };
    // Reordering fields changes the shape, not the contract of a type mods cannot build.
    let reordered = export_fixture(
        &[(
            LIB,
            "    pub parts: Vec<Text>,\n    style: u8,",
            "    style: u8,\n    pub parts: Vec<Text>,",
        )],
        HOOKS,
    );
    assert_ne!(shape(&base, "type:Text"), shape(&reordered, "type:Text"));
    // A nested type's change reaches the types that hold it.
    let nested = export_fixture(&[(LIB, "    pub y: i32,\n}", "    pub y: i64,\n}")], HOOKS);
    let holder = |e: &Export| shape(e, "type:PingEvent");
    assert_eq!(holder(&base), holder(&export_fixture(&[], HOOKS)));
    let with_point = export_fixture(
        &[(
            LIB,
            "pub cancelled: bool,",
            "pub cancelled: bool,\n        pub at: Point,",
        )],
        HOOKS,
    );
    let with_wider_point = export_fixture(
        &[
            (
                LIB,
                "pub cancelled: bool,",
                "pub cancelled: bool,\n        pub at: Point,",
            ),
            (LIB, "    pub y: i32,\n}", "    pub y: i64,\n}"),
        ],
        HOOKS,
    );
    assert_ne!(holder(&with_point), holder(&with_wider_point));
    assert_ne!(shape(&base, "type:Point"), shape(&nested, "type:Point"));
    // Renaming an unrelated type changes no other shape.
    let renamed = export_fixture(&[(LIB, "pub enum Mode {", "pub enum Toggle {")], HOOKS);
    assert_eq!(shape(&base, "type:Text"), shape(&renamed, "type:Text"));
    assert_eq!(shape(&base, "type:Point"), shape(&renamed, "type:Point"));
}

/// The real tree exports with unique ids and every catalog hook.
#[test]
fn the_real_tree_exports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let export = export(&root).unwrap();
    let text = export.surface.to_json();
    let read = Surface::read(text.as_bytes()).unwrap();
    let catalog = Catalog::load(&root).unwrap();
    let ids: BTreeSet<&str> = read.hooks.iter().map(|h| h.id.as_str()).collect();
    for hook in &catalog.hooks {
        assert!(
            ids.contains(format!("hook:{}", hook.name).as_str()),
            "{}",
            hook.name
        );
    }
    assert!(read.traits.iter().any(|t| t.id == "trait:Hooks"));
}
