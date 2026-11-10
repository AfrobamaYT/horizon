//! The removal in a player's break (`ServerPlayerGameMode.destroyBlock`): the harvest answer,
//! then `playerWillDestroy` and `Level.removeBlock`, which the block's parts may take over.
use super::Game;
use horizon_client::items::ItemStack;
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;

impl Game<'_> {
    /// Remove the block `state` at `pos` for the player breaking it with `held`, and say whether
    /// it was removed and whether the break harvests (drops loot). The harvest answer is read
    /// before the removal, as vanilla's `hasCorrectToolForDrops` is. The block's parts
    /// (`on_destroyed_by_player`, `IBlockExtension.onDestroyedByPlayer`) answer first, and with
    /// none answering, `playerWillDestroy` and the removal are vanilla's.
    pub(in crate::game) fn player_destroy_block_removal(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        name: &str,
        requires_tool: bool,
        held: Option<&ItemStack>,
    ) -> (bool, bool) {
        let creative = self.creative;
        let mut correct_tool = !requires_tool || self.res.items().correct_for_drops(held, name);
        let mut harvest = correct_tool;
        if !creative {
            self.mod_harvest_check(pos, state, &mut harvest);
            if self
                .res
                .behaviours()
                .data(state)
                .is(horizon_world::level::classes::Class::Door)
            {
                correct_tool = harvest;
            }
        }
        let mut destroyed = None;
        if crate::game::api_values::block_state::block_listens(
            mod_api::HookId::OnDestroyedByPlayer,
            state,
        ) {
            let player = self.native_acting_id();
            self.native_on_destroyed_by_player(
                player,
                pos,
                state,
                held,
                !creative && harvest,
                &mut destroyed,
            );
        }
        let source = self
            .lan_acting_uuid
            .map(horizon_world::level::LevelEventPlayer);
        let removed = destroyed.unwrap_or_else(|| {
            self.with_level(|b, level, ticks| {
                b.player_destroy_block(
                    level,
                    ticks,
                    BlockPos::new(pos[0], pos[1], pos[2]),
                    creative,
                    correct_tool,
                    source,
                )
            })
        });
        (removed, harvest)
    }
}
