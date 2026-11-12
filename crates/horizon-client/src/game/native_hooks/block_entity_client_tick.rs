//! Client ticks of mod block entities (`ClientLevel.tickBlockEntities`): the client's own
//! ticker list, made from `native_block_entity_client_ticks`.
use super::*;
use horizon_client::block_entities::Ticking;
use horizon_client::containers::BlockEntity;
use mod_api::ModBlockEntity;

impl Game<'_> {
    /// Whether block `block`'s block entity ticks on the client
    /// (`EntityBlock.getTicker` asked with the client level): set `ticks` when
    /// it is yours, and `native_block_entity_client_tick` runs for it once per
    /// client tick. Asked for each of the block's states the first time a
    /// client level ticks with a part that has a client tick.
    /// Set `ticks` to true to make it tick, and never clear a value an earlier mod or mixin set.
    pub(in crate::game) fn native_block_entity_client_ticks(&self, block: &str, ticks: &mut bool) {
        self.fire_block_entity_client_ticks(block, ticks);
    }
    /// A client tick of this client's copy of the mod block entity at `pos`
    /// (`ClientLevel.tickBlockEntities`), when its `id` is yours and its
    /// block's state ticks on the client (`native_block_entity_client_ticks`):
    /// once per client tick while the level runs, in vanilla's ticker order
    /// among the others. `block_entity` is its handle, `state` its block's
    /// state and `entity` its data as the server last sent it
    /// (`native_block_entity_update_tag`), read only. Start, stop and fade
    /// its sounds and add its particles here. Never called on a server, and
    /// never again for a copy removed or whose chunk unloaded.
    pub(in crate::game) fn native_block_entity_client_tick(
        &mut self,
        pos: [i32; 3],
        block_entity: BlockEntityRef,
        state: BlockStateId,
        entity: &ModBlockEntity,
    ) {
        self.fire_block_entity_client_tick(pos, block_entity, state, entity);
    }
}

impl Game<'_> {
    /// `ClientLevel.tickBlockEntities` for the mod block entities of this client's level:
    /// the client's ticker list (`Level.blockEntityTickers`, kept in its world's block
    /// entities as a server keeps its own) holds the copies whose block state has a client
    /// ticker, and each live one is told in its order. Nothing is asked or kept while no
    /// part has a client tick, and nothing runs off a server's client (a window that
    /// simulates the world itself, or a server).
    pub(in crate::game) fn client_tick_mod_block_entities(&mut self) {
        if !mod_api::__private::has_part_hook(mod_api::HookId::BlockEntityClientTick) {
            return;
        }
        let Some(level) = self.native_client_level() else {
            return;
        };
        if !self.world.block_entities.has_states() {
            let states = self.client_ticking_states();
            self.world.block_entities.set_states(states);
        }
        let tickers = self.world.block_entity_tickers();
        for &ticker in tickers.iter() {
            let (x, y, z) = ticker.0;
            // Ticks before this one can remove it (`TickingBlockEntity.isRemoved`).
            if !self.world.block_entities.is_live(ticker)
                || !self.border.contains(f64::from(x), f64::from(z))
            {
                continue;
            }
            let block_entities = &self.world.block_entities;
            let (Some(BlockEntity::Mod(entity)), Some(generation)) = (
                block_entities.get(&ticker.0),
                block_entities.generation(&ticker.0),
            ) else {
                continue;
            };
            let Ok(entity) = entity.try_into_api() else {
                continue;
            };
            let state = self.world.block(x, y, z);
            let handle = mod_api::__private::game::block_entity_ref(level, [x, y, z], generation);
            self.native_block_entity_client_tick([x, y, z], handle, state, &entity);
        }
    }
    /// `BlockState.getTicker` with the client level, for every block state: a mod block's
    /// states tick when a part says so (asked once per block), every other state never.
    fn client_ticking_states(&self) -> Vec<Ticking> {
        let registry = self.pack.assets.registry;
        let mut asked: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
        (0..registry.len())
            .map(|i| {
                let name: &str = &self.res.facts().get(BlockStateId(i as u32)).name;
                if name.starts_with("minecraft:") {
                    return Ticking::No;
                }
                let ticks = *asked.entry(name).or_insert_with(|| {
                    let mut ticks = false;
                    self.native_block_entity_client_ticks(name, &mut ticks);
                    ticks
                });
                if ticks { Ticking::Yes } else { Ticking::No }
            })
            .collect()
    }
}
