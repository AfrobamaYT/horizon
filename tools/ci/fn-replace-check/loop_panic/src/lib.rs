//! A `[[native.loop]]` slot that panics on the tenth hopper it sees: the server crashes, and the
//! crash report names this mod and the loop.
use mod_api::loops::LoopFlow;
use mod_api::loops::block_entity_tick::{BlockEntityTickSlot, BlockEntityTickView, TickedKind};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

#[derive(Default)]
pub struct Tenth {
    hoppers: u32,
}

impl BlockEntityTickSlot for Tenth {
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        if view.kind() == TickedKind::Hopper {
            self.hoppers += 1;
            if self.hoppers == 10 {
                panic!("fnr: loop_panic on the tenth hopper, at {:?}", view.pos());
            }
        }
        LoopFlow::Continue
    }
}
