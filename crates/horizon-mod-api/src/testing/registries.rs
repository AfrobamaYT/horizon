//! The registries fake: item reports, items with slots of their own, and the loaded mods.
use crate::{ItemStack, Tag};
use std::collections::{HashMap, HashSet};

/// The registries a test reads: each item's default components (its item report), the items
/// that hold slots of their own, and the mods loaded beside the one under test.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FakeRegistries {
    components: HashMap<(String, String), Tag>,
    containers: HashMap<String, usize>,
    loaded: HashSet<String>,
}

impl FakeRegistries {
    /// Gives item `item` the default `component` `value`, as its item report would.
    pub fn item_component(&mut self, item: &str, component: &str, value: Tag) -> &mut Self {
        self.components
            .insert((item.to_owned(), component.to_owned()), value);
        self
    }
    /// Gives item `item` `slots` slots of the game's own, as `item_container` answers for a
    /// mod's backpack; they read empty.
    pub fn item_slots(&mut self, item: &str, slots: usize) -> &mut Self {
        self.containers.insert(item.to_owned(), slots);
        self
    }
    /// Marks mod `id` as loaded.
    pub fn loaded(&mut self, id: &str) -> &mut Self {
        self.loaded.insert(id.to_owned());
        self
    }

    /// Item `item`'s default `component`. Every item's report has `minecraft:max_stack_size`,
    /// 64 unless given.
    pub(super) fn default_component(&self, item: &str, component: &str) -> Option<Tag> {
        let given = self
            .components
            .get(&(item.to_owned(), component.to_owned()))
            .cloned();
        given.or_else(|| (component == "minecraft:max_stack_size").then_some(Tag::Int(64)))
    }
    /// How many of item `item` one stack holds.
    pub(super) fn max_stack_size(&self, item: &str) -> u32 {
        match self.default_component(item, "minecraft:max_stack_size") {
            Some(Tag::Int(size)) => u32::try_from(size).unwrap_or(0),
            _ => 64,
        }
    }
    /// The slots `stack` holds, when its item has any.
    pub(super) fn container_slots(&self, stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
        self.containers
            .get(&*stack.item)
            .map(|&slots| vec![None; slots])
    }
    pub(super) fn is_loaded(&self, id: &str) -> bool {
        self.loaded.contains(id)
    }
}
