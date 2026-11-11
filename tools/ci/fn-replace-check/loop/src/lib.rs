//! Two `[[native.loop]]` slots in `block_entities.tick`, compiled into the launcher's copy of the
//! loop, and a HEAD mixin on the loop's entry.
use mod_api::loops::LoopFlow;
use mod_api::loops::block_entity_tick::{BlockEntityTickSlot, BlockEntityTickView, TickedKind};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

/// Called by the HEAD helper.
pub fn note(what: &str) {
    eprintln!("fnr: loop {what}");
}

/// Counts the visits and ticks it sees; always answers Continue.
#[derive(Default)]
pub struct Count {
    visited: u64,
    ticked: u64,
}

impl BlockEntityTickSlot for Count {
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        self.visited += 1;
        if self.visited % 100 == 1 {
            note(&format!(
                "count before #{}: {:?} at {:?} in {:?}, time {}, cooldown {:?}",
                self.visited,
                view.kind(),
                view.pos(),
                view.dimension(),
                view.game_time(),
                view.hopper_cooldown()
            ));
        }
        LoopFlow::Continue
    }
    fn after(&mut self, view: &mut impl BlockEntityTickView) {
        self.ticked += 1;
        if self.ticked % 100 == 1 {
            note(&format!("count after #{}: {:?}", self.ticked, view.kind()));
        }
    }
}

/// While a hopper cools down, takes one tick off its cooldown and skips its tick.
#[derive(Default)]
pub struct Throttle {
    skipped: u64,
}

impl BlockEntityTickSlot for Throttle {
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        match view.hopper_cooldown() {
            Some(cooldown) if cooldown > 1 => {
                view.set_hopper_cooldown(cooldown - 1);
                self.skipped += 1;
                if self.skipped % 100 == 1 {
                    note(&format!(
                        "throttle skips #{}: cooldown {cooldown}",
                        self.skipped
                    ));
                }
                LoopFlow::Skip
            }
            _ => {
                // Off a hopper, the mutator writes nothing.
                if view.kind() != TickedKind::Hopper {
                    assert!(!view.set_hopper_cooldown(5));
                }
                LoopFlow::Continue
            }
        }
    }
}
