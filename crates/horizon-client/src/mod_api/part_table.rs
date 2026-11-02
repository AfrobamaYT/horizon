//! Parts (arismoko/horizon#41): code a mod attaches to one block, block entity, item or mod
//! menu type at startup. The scoped hooks ask the parts attached to the thing they are about,
//! after every mod's `Hooks` method.
use super::{BlockEntityPart, BlockPart, Hooks, ItemPart, MenuPart};
use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    sync::{Arc, LazyLock, OnceLock},
};

/// A part, with the mod that attached it.
pub struct Attached<P: ?Sized> {
    /// The mod's place in dispatch order.
    pub(super) mod_index: usize,
    /// The part's kind: one per thing.
    kind: TypeId,
    pub(super) type_name: &'static str,
    pub(super) part: Arc<P>,
}

/// Every mod's parts, by the id of the thing they are attached to, in dispatch order.
#[derive(Default)]
pub struct Parts {
    /// Block parts, and block entity parts as the block parts they also are.
    blocks: HashMap<Box<str>, Vec<Attached<dyn BlockPart>>>,
    /// Block entity parts by block, for the hooks asked by block (`block_entity`, `screen`).
    block_entities: HashMap<Box<str>, Vec<Attached<dyn BlockEntityPart>>>,
    /// Block entity parts by block entity type, for the hooks asked by the entity
    /// (`block_entity_tick`, `menu_button`): its type id need not be its block's id.
    block_entity_kinds: HashMap<Box<str>, Vec<Attached<dyn BlockEntityPart>>>,
    items: HashMap<Box<str>, Vec<Attached<dyn ItemPart>>>,
    menus: HashMap<Box<str>, Vec<Attached<dyn MenuPart>>>,
}

static PARTS: OnceLock<Parts> = OnceLock::new();
static NO_PARTS: LazyLock<Parts> = LazyLock::new(Parts::default);

/// The installed parts, or none before [`install_parts`].
pub(crate) fn parts() -> &'static Parts {
    PARTS.get().unwrap_or(&NO_PARTS)
}

impl Parts {
    // Each takes the thing's id as a closure: with nothing attached in that scope the id is never
    // worked out, so a game with no parts pays no registry read on the hot hooks.
    pub(crate) fn block<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockPart>] {
        attached(&self.blocks, id)
    }
    pub(crate) fn block_entity<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockEntityPart>] {
        attached(&self.block_entities, id)
    }
    pub(crate) fn block_entity_kind<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockEntityPart>] {
        attached(&self.block_entity_kinds, id)
    }
    pub(crate) fn item<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn ItemPart>] {
        attached(&self.items, id)
    }
    pub(crate) fn menu<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn MenuPart>] {
        attached(&self.menus, id)
    }
}

fn attached<'a, 'k, P: ?Sized>(
    table: &'a HashMap<Box<str>, Vec<Attached<P>>>,
    id: impl FnOnce() -> Option<&'k str>,
) -> &'a [Attached<P>] {
    if table.is_empty() {
        return &[];
    }
    id().and_then(|id| table.get(id)).map_or(&[], Vec::as_slice)
}

/// What a mod's [`Hooks::register`] attaches its parts with. A thing takes one part of each
/// kind: the same kind twice on one thing refuses the mods at startup, naming both.
#[derive(Default)]
pub struct Registrar {
    mod_index: usize,
    parts: Parts,
    refused: Vec<String>,
}

impl Registrar {
    /// Attach `part` to block `block` (`mymod:shaft`, `minecraft:oak_log`).
    pub fn block_part<P: BlockPart>(&mut self, block: &str, part: P) {
        self.attach::<P, dyn BlockPart>(|p| &mut p.blocks, block, Arc::new(part));
    }
    /// Attach `part` to the mod block entity type `kind` (the id a `block_entity` hook gives the
    /// entity, `mymod:press`) and to `blocks`, the blocks that hold one. It answers the hooks
    /// asked by the entity (`block_entity_tick`, `menu_button`), those asked by block
    /// (`block_entity`, `screen`) and every block hook of those blocks.
    pub fn block_entity_part<P: BlockEntityPart>(&mut self, kind: &str, blocks: &[&str], part: P) {
        let part = Arc::new(part);
        if !self.attach::<P, dyn BlockEntityPart>(|p| &mut p.block_entity_kinds, kind, part.clone())
        {
            return;
        }
        for block in blocks {
            // Every block entity part is a block part too, so a second one is found there.
            if self.attach::<P, dyn BlockPart>(|p| &mut p.blocks, block, part.clone()) {
                self.attach::<P, dyn BlockEntityPart>(
                    |p| &mut p.block_entities,
                    block,
                    part.clone(),
                );
            }
        }
    }
    /// Attach `part` to item `item` (`mymod:wrench`).
    pub fn item_part<P: ItemPart>(&mut self, item: &str, part: P) {
        self.attach::<P, dyn ItemPart>(|p| &mut p.items, item, Arc::new(part));
    }
    /// Attach `part` to the mod menu type `menu`.
    pub fn menu_part<P: MenuPart>(&mut self, menu: &str, part: P) {
        self.attach::<P, dyn MenuPart>(|p| &mut p.menus, menu, Arc::new(part));
    }

    /// Attach `part`, of kind `K`, to `id` in `table`; false if it is refused.
    fn attach<K: 'static, P: ?Sized>(
        &mut self,
        table: fn(&mut Parts) -> &mut HashMap<Box<str>, Vec<Attached<P>>>,
        id: &str,
        part: Arc<P>,
    ) -> bool {
        let name = type_name::<K>();
        let namespaced = id
            .split_once(':')
            .is_some_and(|(namespace, path)| !namespace.is_empty() && !path.is_empty());
        if !namespaced {
            self.refused.push(format!(
                "{} attached {name} to {id:?}, which is not a namespaced id",
                super::mod_name(self.mod_index)
            ));
            return false;
        }
        let list = table(&mut self.parts).entry(id.into()).or_default();
        if let Some(other) = list.iter().find(|a| a.kind == TypeId::of::<K>()) {
            self.refused.push(format!(
                "{name} is attached to {id} twice, by {} and {}",
                super::mod_name(other.mod_index),
                super::mod_name(self.mod_index)
            ));
            return false;
        }
        list.push(Attached {
            mod_index: self.mod_index,
            kind: TypeId::of::<K>(),
            type_name: name,
            part,
        });
        true
    }
}

/// Builds the parts from every mod's [`Hooks::register`], in dispatch order. Once, at startup;
/// the error lists every part refused, naming its mods.
pub(crate) fn install_parts(mods: &[Box<dyn Hooks>]) -> Result<(), Vec<String>> {
    let mut registrar = Registrar::default();
    for (index, hooks) in super::entered(mods, "register").enumerate() {
        registrar.mod_index = index;
        hooks.register(&mut registrar);
    }
    if !registrar.refused.is_empty() {
        return Err(registrar.refused);
    }
    if PARTS.set(registrar.parts).is_err() {
        log::warn!("Mod parts already installed; keeping the first table");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;

    /// A comparator reading of 5 more than it was.
    struct Gauge;
    impl BlockPart for Gauge {
        fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
            event.output += 5;
        }
    }

    /// The block entity of type `test:press_entity`, held by block `test:press`: it ticks, and
    /// each tick adds a data slot.
    struct Press;
    impl BlockPart for Press {}
    impl BlockEntityPart for Press {
        fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
            event.ticks = true;
        }
        fn block_entity_tick(&self, _: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
            event.entity.data.push(1);
        }
    }
    struct NoHost;
    impl Host for NoHost {}

    /// Doubles every comparator reading, and attaches `Gauge` to `test:gauge` and `Press` to
    /// `test:press`.
    struct Doubler;
    impl Hooks for Doubler {
        fn register(&self, r: &mut Registrar) {
            r.block_part("test:gauge", Gauge);
            r.block_entity_part("test:press_entity", &["test:press"], Press);
        }
        fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
            event.output *= 2;
        }
    }

    /// State 1 is a `test:gauge`, state 2 a `test:other`.
    struct Level;
    impl LevelHost for Level {
        fn block_state(&self, _: [i32; 3]) -> BlockStateId {
            unimplemented!()
        }
        fn is_client_side(&self) -> bool {
            false
        }
        fn state_with_property(&self, _: BlockStateId, _: &str, _: &str) -> Option<BlockStateId> {
            None
        }
        fn state_block(&self, state: BlockStateId) -> Option<&str> {
            ["test:gauge", "test:other"]
                .get(state.0 as usize - 1)
                .copied()
        }
    }

    #[test]
    fn a_block_part_answers_for_its_block_only_after_the_mods() {
        let mods: Vec<Box<dyn Hooks>> = vec![Box::new(Doubler)];
        install_parts(&mods).unwrap();
        let read = |state| {
            let mut output = 1;
            dispatch_analog_output(
                &Level,
                &mods,
                LevelDimension::Overworld,
                BlockPos::new(0, 64, 0),
                BlockStateId(state),
                Direction::North,
                &mut output,
            );
            output
        };
        // Doubled by the mod, then 5 more from the part: (1 * 2) + 5.
        assert_eq!(read(1), 7);
        assert_eq!(read(2), 2);
    }

    #[test]
    fn a_block_entity_part_answers_by_block_and_by_entity_type() {
        let mods: Vec<Box<dyn Hooks>> = vec![Box::new(Doubler)];
        install_parts(&mods).unwrap();
        let ticks = |block| {
            let mut ticks = false;
            dispatch_block_entity_ticks(&NoHost, &mods, block, &mut ticks);
            ticks
        };
        // The ticker is asked by block: the type id is not a block.
        assert!(ticks("test:press"));
        assert!(!ticks("test:press_entity"));
        let tick = |kind: &str| {
            let mut entity = ModBlockEntity::default();
            entity.id = kind.into();
            dispatch_block_entity_tick(&mut NoHost, &mods, [0, 64, 0], &mut entity);
            entity.data.len()
        };
        // The tick is asked by the entity's type, which differs from its block's id.
        assert_eq!(tick("test:press_entity"), 1);
        assert_eq!(tick("test:press"), 0);
    }

    #[test]
    fn with_no_parts_the_block_is_never_named() {
        let parts = Parts::default();
        let named = std::cell::Cell::new(false);
        let id = || {
            named.set(true);
            Some("test:gauge")
        };
        assert!(parts.block(id).is_empty());
        assert!(!named.get(), "an empty table resolved the state's block");
    }

    #[test]
    fn the_same_part_twice_on_one_block_is_refused() {
        struct Other;
        impl BlockPart for Other {}
        let mut r = Registrar::default();
        r.block_part("test:gauge", Gauge);
        r.mod_index = 1;
        r.block_part("test:gauge", Other);
        r.block_part("test:gauge", Gauge);
        r.block_part("gauge", Other);
        assert_eq!(r.parts.block(|| Some("test:gauge")).len(), 2);
        assert_eq!(r.refused.len(), 2, "{:?}", r.refused);
        assert!(r.refused[0].contains("Gauge is attached to test:gauge twice"));
        assert!(r.refused[1].contains("not a namespaced id"));
    }
}
