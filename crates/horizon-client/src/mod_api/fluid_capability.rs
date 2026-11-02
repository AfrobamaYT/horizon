//! The fluid capability (arismoko/horizon#41 §7): [`FLUIDS`], as NeoForge 26.3's
//! `Capabilities.Fluid.BLOCK`, in the shape of [`ITEMS`](super::ITEMS). Amounts are millibuckets:
//! a bucket is 1000.
use super::{CapabilityKey, Direction, Host};
use crate::item_data::StackData;
use std::{
    collections::HashSet,
    marker::PhantomData,
    sync::{Arc, LazyLock, Mutex},
};

/// An amount of one fluid, as NeoForge's `FluidStack`: its id (`minecraft:water`), how many
/// millibuckets, and its data components, kept as an item stack's are (a potion fluid's
/// `minecraft:potion_contents`). Water and lava have none.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidStack {
    pub fluid: Arc<str>,
    pub amount: u32,
    pub components: Option<Box<StackData>>,
}

impl FluidStack {
    /// `amount` of `fluid`, with no components.
    pub fn new(fluid: &str, amount: u32) -> Self {
        Self {
            fluid: Arc::from(fluid),
            amount,
            components: None,
        }
    }
    /// The same fluid with the same components, `amount` of it.
    pub fn with_amount(&self, amount: u32) -> Self {
        Self {
            amount,
            ..self.clone()
        }
    }
    /// Whether it carries any component.
    pub fn has_components(&self) -> bool {
        self.components
            .as_deref()
            .is_some_and(|components| *components != StackData::default())
    }
    /// NeoForge's `FluidStack.isSameFluidSameComponents`: the same id and the same components,
    /// an empty patch counting as none. Fills, merges and capacities ask it.
    pub fn same_fluid_same_components(&self, other: &Self) -> bool {
        let none = StackData::default();
        self.fluid == other.fluid
            && self.components.as_deref().unwrap_or(&none)
                == other.components.as_deref().unwrap_or(&none)
    }
}

/// Fluid tanks, as NeoForge 26.3's `Capabilities.Fluid.BLOCK` (a `ResourceHandler<FluidResource>`).
/// The game answers it for a mod block's tanks (`block_entity_fluids`) and for a cauldron, one
/// tank of 1000 mB that holds water in thirds and lava only whole; a cauldron has no block
/// entity, so it is answered by position. Powder snow is no fluid: its cauldron has no answer.
pub const FLUIDS: CapabilityKey<dyn FluidHandler> = CapabilityKey {
    name: "horizon:fluids",
    default: Some(|host, pos| {
        static GAME_FLUIDS: LazyLock<Arc<dyn FluidHandler>> =
            LazyLock::new(|| Arc::new(GameFluids));
        host.fluid_tank_count(pos, None)
            .map(|_| GAME_FLUIDS.clone())
    }),
    handler: PhantomData,
};

/// The tanks of the block at `at` as its side `side` reaches them (none: every tank), numbered
/// from 0 for that side, as [`ItemHandler`](super::ItemHandler)'s slots. `simulate` changes
/// nothing and must answer exactly as the real call would, as NeoForge's handlers must: a move
/// plans with it at both ends, then commits.
pub trait FluidHandler: Send + Sync + 'static {
    /// How many tanks `side` reaches.
    fn tanks(&self, host: &dyn Host, at: [i32; 3], side: Option<Direction>) -> usize;
    /// What tank `tank` holds, if anything.
    fn fluid(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
    ) -> Option<FluidStack>;
    /// How much of `fluid` tank `tank` holds when empty, whatever it holds now: none where
    /// `fluid` may not go in, such as a drain-only tank. NeoForge's `isValid` and
    /// `getCapacityAsLong` in one.
    fn capacity(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        fluid: &FluidStack,
    ) -> u32;
    /// Puts as much of `fluid` as tank `tank` takes, into an empty tank or onto the same fluid
    /// with the same components; what didn't fit.
    fn insert(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        fluid: FluidStack,
        simulate: bool,
    ) -> Option<FluidStack>;
    /// Takes up to `max` out of tank `tank`; what came out.
    fn extract(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        max: u32,
        simulate: bool,
    ) -> Option<FluidStack>;
}

/// The game's own [`FLUIDS`]: a mod block's tanks by its table's sides, and cauldrons.
struct GameFluids;
impl FluidHandler for GameFluids {
    fn tanks(&self, host: &dyn Host, at: [i32; 3], side: Option<Direction>) -> usize {
        host.fluid_tank_count(at, side).unwrap_or(0)
    }
    fn fluid(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
    ) -> Option<FluidStack> {
        host.fluid_tank(at, side, tank)
    }
    fn capacity(
        &self,
        host: &dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        fluid: &FluidStack,
    ) -> u32 {
        host.fluid_tank_capacity(at, side, tank, fluid)
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        fluid: FluidStack,
        simulate: bool,
    ) -> Option<FluidStack> {
        host.fluid_tank_insert(at, side, tank, fluid, simulate)
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        at: [i32; 3],
        side: Option<Direction>,
        tank: usize,
        max: u32,
        simulate: bool,
    ) -> Option<FluidStack> {
        host.fluid_tank_extract(at, side, tank, max, simulate)
    }
}

/// The owners of fluid handlers whose real calls disagreed with their simulation, warned once.
static BROKEN: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Who registered `handler`: a mod's id, or the game.
fn owner(handler: &Arc<dyn FluidHandler>) -> String {
    super::capability_mod(handler).map_or_else(|| "the game".to_owned(), super::mod_name)
}

/// A handler's real call disagreed with its simulation: a mod bug, warned once per owner.
fn broken(handler: &Arc<dyn FluidHandler>, at: [i32; 3], what: &str) {
    let owner = owner(handler);
    let mut warned = BROKEN.lock().unwrap_or_else(|e| e.into_inner());
    if warned.get_or_insert_default().insert(owner.clone()) {
        log::warn!(
            "Horizon: {owner}'s fluid handler at {at:?} {what} other than its simulation said. \
             Its simulate must answer as its real call does."
        );
    }
}

#[cfg(test)]
pub(super) fn warned_about(owner: &str) -> bool {
    let broken = BROKEN.lock().unwrap_or_else(|e| e.into_inner());
    broken.as_ref().is_some_and(|owners| owners.contains(owner))
}

/// One end of a move: a block's handler as one side reaches it.
struct End {
    handler: Arc<dyn FluidHandler>,
    at: [i32; 3],
    side: Option<Direction>,
}

impl End {
    /// How `fluid` would go into the tanks, simulated tank by tank: how much into which.
    fn plan_fill(&self, host: &mut dyn Host, fluid: &FluidStack) -> Vec<(usize, u32)> {
        let mut plan = Vec::new();
        let mut left = fluid.amount;
        for tank in 0..self.handler.tanks(host, self.at, self.side) {
            if left == 0 {
                break;
            }
            let some = fluid.with_amount(left);
            let rest = self
                .handler
                .insert(host, self.at, self.side, tank, some, true);
            let rest = rest.map_or(0, |rest| rest.amount.min(left));
            if rest < left {
                plan.push((tank, left - rest));
                left = rest;
            }
        }
        plan
    }
    /// Fills the tanks as `plan` says; how much went in.
    fn fill(&self, host: &mut dyn Host, fluid: &FluidStack, plan: Vec<(usize, u32)>) -> u32 {
        let mut went = 0;
        for (tank, amount) in plan {
            let some = fluid.with_amount(amount);
            let rest = self
                .handler
                .insert(host, self.at, self.side, tank, some, false);
            went += amount - rest.map_or(0, |rest| rest.amount.min(amount));
        }
        went
    }
}

/// How much a plan puts in.
fn planned(plan: &[(usize, u32)]) -> u32 {
    plan.iter().map(|&(_, amount)| amount).sum()
}

impl dyn Host + '_ {
    /// Moves up to `max` millibuckets of one fluid from the block at `from`, through its side
    /// `from_side`, into the one at `to`, through `to_side`, by their [`FLUIDS`] handlers, as
    /// NeoForge's `ResourceHandlerUtil.move`. Each source tank in turn is planned with
    /// `simulate` at both ends (what the source gives, what the target takes of it, and what
    /// the source gives of that, since a cauldron gives only whole thirds), then moved. What
    /// moved; none when nothing did.
    ///
    /// A handler whose real call disagrees with its simulation is a mod bug, warned once naming
    /// its mod. What it leaves over goes back into the source tank, else into the target. Fluid
    /// has no item form to drop at the block, so what neither end takes is lost, with a warning
    /// naming the mod: the one thing a broken handler can void.
    pub fn transfer_fluids(
        &mut self,
        from: [i32; 3],
        from_side: Option<Direction>,
        to: [i32; 3],
        to_side: Option<Direction>,
        max: u32,
    ) -> Option<FluidStack> {
        if from == to {
            return None;
        }
        let source = End {
            handler: self.capability(&FLUIDS, from)?,
            at: from,
            side: from_side,
        };
        let target = End {
            handler: self.capability(&FLUIDS, to)?,
            at: to,
            side: to_side,
        };
        let mut moved: Option<FluidStack> = None;
        for tank in 0..source.handler.tanks(self, from, from_side) {
            let left = max - moved.as_ref().map_or(0, |moved| moved.amount);
            if left == 0 {
                break;
            }
            let extract = |host: &mut dyn Host, max, simulate| {
                source
                    .handler
                    .extract(host, from, from_side, tank, max, simulate)
                    .filter(|out| out.amount > 0)
            };
            let Some(offer) = extract(self, left, true) else {
                continue;
            };
            if moved
                .as_ref()
                .is_some_and(|moved| !moved.same_fluid_same_components(&offer))
            {
                continue;
            }
            let takes = planned(&target.plan_fill(self, &offer));
            let Some(give) = (takes > 0).then(|| extract(self, takes, true)).flatten() else {
                continue;
            };
            let plan = target.plan_fill(self, &give);
            if planned(&plan) != give.amount {
                continue;
            }
            // Commit what was planned.
            let Some(out) = extract(self, give.amount, false) else {
                broken(&source.handler, from, "gave");
                continue;
            };
            let as_planned = out == give;
            let mut liar = (!as_planned).then_some(&source.handler);
            let plan = if as_planned {
                plan
            } else {
                broken(&source.handler, from, "gave");
                target.plan_fill(self, &out)
            };
            let went = target.fill(self, &out, plan);
            if went < out.amount && as_planned {
                broken(&target.handler, to, "took");
                liar = Some(&target.handler);
            }
            if went < out.amount {
                let rest = out.with_amount(out.amount - went);
                let back = source
                    .handler
                    .insert(self, from, from_side, tank, rest, false);
                let lost = back.filter(|back| back.amount > 0).and_then(|back| {
                    let plan = target.plan_fill(self, &back);
                    let into = target.fill(self, &back, plan);
                    (into < back.amount).then(|| back.with_amount(back.amount - into))
                });
                if let Some(lost) = lost {
                    let owner = liar.map_or_else(|| "the game".to_owned(), owner);
                    log::warn!(
                        "Horizon: a fluid move from {from:?} to {to:?} lost {} mB of {}: \
                         {owner}'s fluid handler moved other than its simulation said",
                        lost.amount,
                        lost.fluid
                    );
                }
            }
            if went > 0 {
                let total = moved.as_ref().map_or(0, |moved| moved.amount) + went;
                moved = Some(out.with_amount(total));
            }
        }
        moved
    }
}
