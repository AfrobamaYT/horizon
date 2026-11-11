use super::*;

impl Game<'_> {
    /// A replacement can inherit a ticker slot later in the current pass.
    /// Deliver its attachment notice before taking it out for that tick.
    pub(in crate::game) fn tick_mod_block_entity(&mut self, pos: Pos) {
        self.fire_pending_block_entity_loads();
        self.tick_mod_block_entity_loaded(pos);
    }

    /// Drain attachment notices before either logical side runs block entity tickers.
    pub(in crate::game) fn fire_pending_block_entity_loads(&mut self) {
        if !self.world.block_entities.has_pending_on_load() {
            return;
        }
        let level = if self.is_logical_server() {
            self.level(self.dim.id())
        } else {
            self.native_client_level()
        };
        let Some(level) = level else { return };
        while let Some((pos, _)) = self.world.block_entities.next_on_load() {
            if !mod_api::__private::has_part_hook(mod_api::HookId::BlockEntityOnLoad) {
                continue;
            }
            let Some(entity) = self.block_entity(level, [pos.0, pos.1, pos.2]) else {
                continue;
            };
            let Some(id) = self.block_entity_kind(entity) else {
                continue;
            };
            self.native_block_entity_on_load(mod_api::__private::BlockEntityOnLoadCall {
                id,
                entity,
            });
        }
    }

    /// Notify subscribers after this instance is attached and its saved data is applied.
    pub(in crate::game) fn native_block_entity_on_load(
        &mut self,
        call: mod_api::__private::BlockEntityOnLoadCall,
    ) {
        self.fire_block_entity_on_load(call);
    }
}
