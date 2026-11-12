/// Counts this mod's counters among the block entities the loop visits, and always answers Continue.
pub struct Own {
    own: OwnBlockEntities,
    counted: u64,
}

impl Default for Own {
    fn default() -> Self {
        Self {
            own: OwnBlockEntities::new(&["fnr_loop_second:counter"]),
            counted: 0,
        }
    }
}

impl BlockEntityTickSlot for Own {
    fn before(&mut self, tick: &mut BlockEntityTick<'_, '_>) -> LoopFlow {
        if self.own.contains(tick) {
            self.counted += 1;
            if self.counted % 200 == 1 {
                // Read the id only to show it: the check above did no string work.
                if let Some(BlockEntity::Mod(entity)) = tick.block_entity() {
                    fnr_loop_second::note(&format!("own #{}: {}", self.counted, entity.id()));
                }
            }
        }
        LoopFlow::Continue
    }
}
