//! NeoForge's built-in block data maps (`NeoForgeDataMaps`): `neoforge:flammable`,
//! `neoforge:oxidizables` and `neoforge:waxables`, as the open world's data packs
//! give them, with the inverse maps `DataMapHooks` builds for scraping and
//! unwaxing. Installed on [`Behaviours`] for the open world, as its live
//! features are; without files nothing is installed and every reader takes
//! vanilla's own answer.
use super::Behaviours;
use crate::registry::BlockStateId;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The three maps by block id (`minecraft:stone`), values decoded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMaps {
    flammable: HashMap<String, (i32, i32)>,
    oxidizes_to: HashMap<String, String>,
    scrapes_to: HashMap<String, String>,
    waxes_to: HashMap<String, String>,
    unwaxes_to: HashMap<String, String>,
}

/// `minecraft:` for an id written without a namespace.
fn full(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

impl BlockMaps {
    /// The maps from their merged values (block id to the value a file gave
    /// it). A value of another shape is skipped with a warning, as a codec
    /// error leaves the entry out.
    pub fn from_values<'a>(
        flammable: impl IntoIterator<Item = (&'a str, &'a Value)>,
        oxidizables: impl IntoIterator<Item = (&'a str, &'a Value)>,
        waxables: impl IntoIterator<Item = (&'a str, &'a Value)>,
    ) -> Self {
        let mut maps = Self::default();
        let int = |v: &Value| v.as_i64().and_then(|n| i32::try_from(n).ok());
        for (block, value) in flammable {
            match (int(&value["ignite_odds"]), int(&value["burn_odds"])) {
                (Some(ignite), Some(burn)) => {
                    maps.flammable.insert(block.to_owned(), (ignite, burn));
                }
                _ => log::warn!("Horizon: neoforge:flammable {block}: not a Flammable, skipped"),
            }
        }
        for (block, value) in oxidizables {
            match value["next_oxidation_stage"].as_str() {
                Some(next) => {
                    maps.scrapes_to.insert(full(next), block.to_owned());
                    maps.oxidizes_to.insert(block.to_owned(), full(next));
                }
                None => {
                    log::warn!("Horizon: neoforge:oxidizables {block}: not an Oxidizable, skipped")
                }
            }
        }
        for (block, value) in waxables {
            match value["waxed"].as_str() {
                Some(waxed) => {
                    maps.unwaxes_to.insert(full(waxed), block.to_owned());
                    maps.waxes_to.insert(block.to_owned(), full(waxed));
                }
                None => log::warn!("Horizon: neoforge:waxables {block}: not a Waxable, skipped"),
            }
        }
        maps
    }
    /// Whether no map gives any block a value.
    pub fn is_empty(&self) -> bool {
        self.flammable.is_empty() && self.oxidizes_to.is_empty() && self.waxes_to.is_empty()
    }
    /// `neoforge:flammable`: the block's ignite and burn odds.
    pub fn flammable(&self, block: &str) -> Option<(i32, i32)> {
        self.flammable.get(block).copied()
    }
    /// `DataMapHooks.getNextOxidizedStage`: the block one oxidation step on.
    pub fn next_oxidation(&self, block: &str) -> Option<&str> {
        self.oxidizes_to.get(block).map(String::as_str)
    }
    /// `DataMapHooks.getPreviousOxidizedStage`: the block one step back (scraping).
    pub fn previous_oxidation(&self, block: &str) -> Option<&str> {
        self.scrapes_to.get(block).map(String::as_str)
    }
    /// `DataMapHooks.getBlockWaxed`: the block's waxed twin.
    pub fn waxed(&self, block: &str) -> Option<&str> {
        self.waxes_to.get(block).map(String::as_str)
    }
    /// `DataMapHooks.getBlockUnwaxed`: the block a waxed one unwaxes to.
    pub fn unwaxed(&self, block: &str) -> Option<&str> {
        self.unwaxes_to.get(block).map(String::as_str)
    }
}

/// The open world's maps on [`Behaviours`]: replaced whole by each data load.
#[derive(Default)]
pub(super) struct Slot(RwLock<Option<Arc<BlockMaps>>>);

impl Behaviours {
    /// Install the open world's NeoForge block maps, replacing the last
    /// load's; `None` where its packs give none.
    pub fn set_block_maps(&self, maps: Option<Arc<BlockMaps>>) {
        *self
            .block_maps
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = maps;
    }
    /// The installed maps. Take them once per operation and reuse them.
    pub fn block_maps(&self) -> Option<Arc<BlockMaps>> {
        self.block_maps
            .0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
    /// `read` over the installed maps without taking a reference count: the
    /// per-block readers' path, one uncontended lock read when none are installed.
    pub(crate) fn read_block_maps<R>(
        &self,
        read: impl FnOnce(&BlockMaps) -> Option<R>,
    ) -> Option<R> {
        self.block_maps
            .0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_deref()
            .and_then(read)
    }
    /// `HoneycombItem.getWaxed` through `DataMapHooks.getBlockWaxed`: the
    /// `neoforge:waxables` twin of `state`'s block, its properties kept; none
    /// where the installed maps give the block none.
    pub fn mapped_waxed(&self, state: BlockStateId) -> Option<BlockStateId> {
        self.read_block_maps(|maps| {
            let twin = self
                .table
                .block_by_name(maps.waxed(self.table.name(state))?)?;
            Some(self.table.with_properties_of(twin, state))
        })
    }
    /// Whether `state` is a waxed block: vanilla's `waxed_` blocks, and every
    /// block the installed `neoforge:waxables` map waxes to.
    pub fn is_waxed(&self, state: BlockStateId) -> bool {
        let name = self.table.name(state);
        name.starts_with("minecraft:waxed_")
            || self
                .read_block_maps(|maps| maps.unwaxed(name).map(drop))
                .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn values_decode_and_the_inverse_maps_follow() {
        let flammable = json!({"ignite_odds": 5, "burn_odds": 20});
        let bad = json!({"ignite_odds": 5});
        let oxidizes = json!({"next_oxidation_stage": "test:tin_dull"});
        let waxes = json!({"waxed": "test:waxed_tin"});
        let maps = BlockMaps::from_values(
            [("minecraft:stone", &flammable), ("minecraft:dirt", &bad)],
            [("test:tin", &oxidizes)],
            [("test:tin", &waxes)],
        );
        assert_eq!(maps.flammable("minecraft:stone"), Some((5, 20)));
        assert_eq!(maps.flammable("minecraft:dirt"), None);
        assert_eq!(maps.next_oxidation("test:tin"), Some("test:tin_dull"));
        assert_eq!(maps.previous_oxidation("test:tin_dull"), Some("test:tin"));
        assert_eq!(maps.waxed("test:tin"), Some("test:waxed_tin"));
        assert_eq!(maps.unwaxed("test:waxed_tin"), Some("test:tin"));
        assert!(!maps.is_empty() && BlockMaps::default().is_empty());
    }
}
