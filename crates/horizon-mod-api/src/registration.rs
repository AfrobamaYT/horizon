//! A wasm mod's [`Hooks::register`](crate::Hooks::register), as it crosses: the mod runs it
//! against a registrar of its own, which checks it exactly as the game's does, and hands over
//! what it attached; the game replays that on its registrar, each part a proxy that calls back
//! into the mod by the index the mod keeps it under.

/// Where a part is attached.
pub enum PartAt {
    /// A block (`Registrar::block_part`).
    Block,
    /// A block entity type (`Registrar::block_entity_part`).
    BlockEntity,
    /// An item (`Registrar::item_part`).
    Item,
    /// A mod menu type (`Registrar::menu_part`).
    Menu,
    /// Every block in a tag (`Registrar::block_part_tag`).
    BlockTag,
    /// Every item in a tag (`Registrar::item_part_tag`).
    ItemTag,
}

/// One part the mod attached, in its registration order.
pub struct RegisteredPart {
    /// Where it is attached.
    pub at: PartAt,
    /// The block, item, menu, block entity type or tag it is attached to.
    pub key: String,
    /// Its Rust type in the mod: one part of each type per thing.
    pub type_name: String,
    /// The index the mod keeps it under, which the game's calls to it carry.
    pub handler: u32,
}

/// A block entity type the mod declared.
pub struct RegisteredType {
    /// Its namespaced id.
    pub id: String,
    /// The blocks that may hold it.
    pub blocks: Vec<String>,
}

/// A ticket controller the mod registered.
pub struct RegisteredController {
    /// Its namespaced id.
    pub id: String,
    /// Whether it validates its saved tickets.
    pub validates: bool,
}

/// Everything one mod's `register` attached, declared and asked for.
#[derive(Default)]
pub struct Registered {
    /// Its parts, in registration order.
    pub parts: Vec<RegisteredPart>,
    /// The block entity types it declared.
    pub types: Vec<RegisteredType>,
    /// Other mods' or vanilla block entity types it looked up.
    pub requested: Vec<String>,
    /// Its ticket controllers.
    pub controllers: Vec<RegisteredController>,
    /// What its registrar refused, each naming the mod.
    pub refused: Vec<String>,
}

/// What a wasm mod exports for its registration; the game's side of the call is
/// `wasm_host::register`.
pub trait Registration {
    /// Runs the mod's `register` as mod `mod_id`, and gives what it attached.
    fn registered(&self, mod_id: String) -> Registered;
}
