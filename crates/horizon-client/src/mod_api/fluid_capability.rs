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
    /// One pass of `fluid` over the tanks, each offered what is left: how much went in. With
    /// `simulate`, how much such a pass would take.
    fn pass(&self, host: &mut dyn Host, fluid: &FluidStack, simulate: bool) -> u32 {
        let mut left = fluid.amount;
        for tank in 0..self.handler.tanks(host, self.at, self.side) {
            if left == 0 {
                break;
            }
            let some = fluid.with_amount(left);
            let rest = self
                .handler
                .insert(host, self.at, self.side, tank, some, simulate);
            left = rest.map_or(0, |rest| rest.amount.min(left));
        }
        fluid.amount - left
    }
    /// Fills the tanks with `fluid` in passes until one takes nothing, as `Inventory.add` and
    /// NeoForge's insert loops do: the first pass makes exactly the offers a simulated `pass`
    /// of `fluid` makes. How much the first pass took, and how much went in.
    fn fill(&self, host: &mut dyn Host, fluid: &FluidStack) -> (u32, u32) {
        let first = self.pass(host, fluid, false);
        let mut went = first;
        while went < fluid.amount {
            let more = self.pass(host, &fluid.with_amount(fluid.amount - went), false);
            if more == 0 {
                break;
            }
            went += more;
        }
        (first, went)
    }
    /// `fill` into tank `tank` alone, in passes; what is left.
    fn fill_tank(&self, host: &mut dyn Host, tank: usize, fluid: FluidStack) -> Option<FluidStack> {
        let mut left = fluid;
        loop {
            let amount = left.amount;
            let rest = self
                .handler
                .insert(host, self.at, self.side, tank, left.clone(), false);
            match rest.filter(|rest| rest.amount > 0) {
                Some(rest) if rest.amount < amount => left = rest,
                Some(_) => return Some(left),
                None => return None,
            }
        }
    }
}

impl dyn Host + '_ {
    /// Moves up to `max` millibuckets of one fluid from the block at `from`, through its side
    /// `from_side`, into the one at `to`, through `to_side`, by their [`FLUIDS`] handlers, as
    /// NeoForge's `ResourceHandlerUtil.move`. Each source tank in turn is planned with
    /// `simulate` at both ends (what the source gives, what the target takes of it in one
    /// pass, and what the source gives of that, since a cauldron gives only whole thirds),
    /// then moved with the same requests: the source's real extract asks what its simulation
    /// asked, and the target is filled in passes until one takes nothing, its first pass
    /// making the simulated offers. What the target doesn't take goes back into the source.
    /// What moved; none when nothing did.
    ///
    /// A handler whose real call disagrees with its simulation is a mod bug, warned once naming
    /// its mod. What is left over goes back into the source tank, else the source, else the
    /// target. Fluid has no item form to drop at the block, so what none of them takes is
    /// lost, with a warning: the one thing a broken handler can void.
    pub fn transfer_fluids(
        &mut self,
        from: [i32; 3],
        from_side: Option<Direction>,
        to: [i32; 3],
        to_side: Option<Direction>,
        max: u32,
    ) -> Option<FluidStack> {
        self.transfer_fluids_where(from, from_side, to, to_side, max, &|_| true)
    }

    /// `transfer_fluids` of the fluids `keep` accepts: the API 55 move, which names a fluid by
    /// its id, keeps those with components where they are.
    pub(crate) fn transfer_fluids_where(
        &mut self,
        from: [i32; 3],
        from_side: Option<Direction>,
        to: [i32; 3],
        to_side: Option<Direction>,
        max: u32,
        keep: &dyn Fn(&FluidStack) -> bool,
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
            let Some(offer) = extract(self, left, true).filter(|offer| keep(offer)) else {
                continue;
            };
            if moved
                .as_ref()
                .is_some_and(|moved| !moved.same_fluid_same_components(&offer))
            {
                continue;
            }
            let ask = target.pass(self, &offer, true);
            let Some(give) = (ask > 0).then(|| extract(self, ask, true)).flatten() else {
                continue;
            };
            let takes = target.pass(self, &give, true);
            if takes == 0 {
                continue;
            }
            // Commit, with the requests that were simulated.
            let Some(out) = extract(self, ask, false) else {
                broken(&source.handler, from, "gave");
                continue;
            };
            let as_planned = out == give;
            let mut liar = None;
            if !as_planned {
                broken(&source.handler, from, "gave");
                liar = Some(&source.handler);
            }
            let (first, went) = target.fill(self, &out);
            if as_planned && first < takes {
                broken(&target.handler, to, "took");
                liar = Some(&target.handler);
            }
            if went < out.amount {
                let rest = out.with_amount(out.amount - went);
                let lost = source
                    .fill_tank(self, tank, rest)
                    .and_then(|rest| {
                        let (_, back) = source.fill(self, &rest);
                        (back < rest.amount).then(|| rest.with_amount(rest.amount - back))
                    })
                    .and_then(|rest| {
                        let (_, into) = target.fill(self, &rest);
                        (into < rest.amount).then(|| rest.with_amount(rest.amount - into))
                    });
                if let Some(lost) = lost {
                    let why = liar.map_or_else(
                        || "neither end took it back".to_owned(),
                        |liar| {
                            format!(
                                "{}'s fluid handler moved other than its simulation said",
                                owner(liar)
                            )
                        },
                    );
                    log::warn!(
                        "Horizon: a fluid move from {from:?} to {to:?} lost {} mB of {}: {why}",
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
