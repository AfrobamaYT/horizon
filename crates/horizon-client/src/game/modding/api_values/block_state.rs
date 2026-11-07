//! Owning-thread, load-bound numeric bridge. Workers receive prepared values instead.
use super::{TryIntoApi, TryIntoEngine};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use mod_api::{BlockState, BlockStateError};
use std::cell::RefCell;

struct Index {
    forward: Vec<BlockState>,
    reverse: std::collections::HashMap<mod_api::BlockKey, Vec<Option<BlockStateId>>>,
}
thread_local! {
    static INDEX: RefCell<Option<Index>> = const { RefCell::new(None) };
}

pub(super) fn clear() {
    INDEX.with_borrow_mut(|index| *index = None);
}
pub(super) fn snapshot() -> Result<Box<[BlockState]>, BlockStateError> {
    INDEX.with_borrow(|index| {
        index
            .as_ref()
            .map(|index| index.forward.clone().into_boxed_slice())
            .ok_or(BlockStateError::Unavailable)
    })
}
pub(super) fn bind(registry: &BlockRegistry) -> Result<(), String> {
    clear();
    let forward = (0..registry.len())
        .map(|id| {
            registry
                .state(BlockStateId(id as u32))
                .expect("registry index")
                .try_into_api()
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("block state index: {error:?}"))?;
    let mut reverse = std::collections::HashMap::<_, Vec<Option<BlockStateId>>>::new();
    for (id, state) in forward.iter().enumerate() {
        let states = reverse.entry(state.block()).or_default();
        states.resize(states.len().max(state.ordinal() as usize + 1), None);
        if states[state.ordinal() as usize]
            .replace(BlockStateId(id as u32))
            .is_some()
        {
            return Err("duplicate block state in numeric registry".into());
        }
    }
    INDEX.with_borrow_mut(|index| *index = Some(Index { forward, reverse }));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_block_index_reorders_refuses_and_clears_failed_rebind() {
        mod_api::__private::part_tests::install_parts_fixture(&[]).unwrap();
        let facts = r#"{"defaults":{"minecraft:air":0,"minecraft:stairs":2},"states":[["a",true,false,"","minecraft:air"],["s",false,false,"",{"id":"minecraft:stairs","properties":{"facing":"north"}}],["s",false,false,"",{"id":"minecraft:stairs","properties":{"facing":"east"}}]]}"#;
        let first = BlockRegistry::from_facts_json(facts.as_bytes()).unwrap();
        super::super::resolve_block_definitions(&first).unwrap();
        let north = BlockStateId(1).try_into_api().unwrap();
        let east = BlockStateId(2).try_into_api().unwrap();
        assert_eq!((north.ordinal(), east.ordinal()), (0, 1));
        assert_eq!(east.try_into_engine(&()), Ok(BlockStateId(2)));
        assert_eq!(
            BlockStateId(u32::MAX).try_into_api(),
            Err(BlockStateError::UnknownState)
        );
        let bad = BlockState::from(mod_api::BlockStateRecord::new(east.block(), u16::MAX));
        assert_eq!(bad.try_into_engine(&()), Err(BlockStateError::UnknownState));
        std::thread::spawn(|| {
            assert_eq!(
                BlockStateId(0).try_into_api(),
                Err(BlockStateError::Unavailable)
            )
        })
        .join()
        .unwrap();
        let reordered = facts
            .replace("north", "TEMP")
            .replace("east", "north")
            .replace("TEMP", "east");
        let reordered = BlockRegistry::from_facts_json(reordered.as_bytes()).unwrap();
        super::super::resolve_block_definitions(&reordered).unwrap();
        assert_eq!(BlockStateId(1).try_into_api(), Ok(east));
        assert_eq!(north.try_into_engine(&()), Ok(BlockStateId(2)));
        assert_eq!(snapshot().unwrap()[1], east);
        let broken = facts.replace("east", "south");
        let broken = BlockRegistry::from_facts_json(broken.as_bytes()).unwrap();
        assert!(super::super::resolve_block_definitions(&broken).is_err());
        assert_eq!(east.try_into_engine(&()), Err(BlockStateError::Unavailable));
        assert_eq!(
            BlockStateId(1).try_into_api(),
            Err(BlockStateError::Unavailable)
        );
        assert!(snapshot().is_err());
    }
}
impl TryIntoApi for BlockStateId {
    type Api = BlockState;
    type Error = BlockStateError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        INDEX.with_borrow(|index| {
            index
                .as_ref()
                .ok_or(BlockStateError::Unavailable)?
                .forward
                .get(self.0 as usize)
                .copied()
                .ok_or(BlockStateError::UnknownState)
        })
    }
}
impl<C: ?Sized> TryIntoEngine<C> for BlockState {
    type Engine = BlockStateId;
    type Error = BlockStateError;
    fn try_into_engine(&self, _: &C) -> Result<Self::Engine, Self::Error> {
        INDEX.with_borrow(|index| {
            let index = index.as_ref().ok_or(BlockStateError::Unavailable)?;
            let states = index
                .reverse
                .get(&self.block())
                .filter(|states| !states.is_empty())
                .ok_or(BlockStateError::UnknownBlock)?;
            states
                .get(self.ordinal() as usize)
                .copied()
                .flatten()
                .ok_or(BlockStateError::UnknownState)
        })
    }
}
