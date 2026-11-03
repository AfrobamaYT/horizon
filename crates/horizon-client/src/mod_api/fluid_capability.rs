//! The fluid capability (arismoko/horizon#41 §7): [`FLUIDS`], as NeoForge 26.3's
//! `Capabilities.Fluid.BLOCK`, a [`ResourceHandler`] of [`FluidResource`]s as
//! [`ITEMS`](super::ITEMS) is of items. Amounts are millibuckets: a bucket is 1000.
use super::{BlockSide, CapabilityKey, Host, ResourceHandler, Transaction};
use crate::item_data::StackData;
use std::{fmt, marker::PhantomData, sync::Arc};

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

/// A fluid and its components, without an amount, as NeoForge's `FluidResource`: what a tank
/// holds. An empty tank holds none.
#[derive(Clone)]
pub struct FluidResource(FluidStack);

impl FluidResource {
    /// What `stack` is, whatever its amount.
    pub fn of(stack: &FluidStack) -> Self {
        Self(stack.with_amount(1))
    }
    /// The fluid's id (`minecraft:water`).
    pub fn fluid(&self) -> &str {
        &self.0.fluid
    }
    /// `amount` millibuckets of it as a stack.
    pub fn to_stack(&self, amount: u32) -> FluidStack {
        self.0.with_amount(amount)
    }
    /// Whether `stack` is this fluid with these components.
    pub fn matches(&self, stack: &FluidStack) -> bool {
        self.0.same_fluid_same_components(stack)
    }
}

impl PartialEq for FluidResource {
    fn eq(&self, other: &Self) -> bool {
        self.matches(&other.0)
    }
}
impl Eq for FluidResource {}

impl fmt::Debug for FluidResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FluidResource").field(&self.0.fluid).finish()
    }
}

/// Fluid tanks, as NeoForge 26.3's `Capabilities.Fluid.BLOCK`: a block's tanks as one side
/// reaches them, numbered from 0 for that side, as [`ITEMS`](super::ITEMS)' slots. The game
/// answers it for a mod block's tanks (`block_entity_fluids`) and for a cauldron, one tank of
/// 1000 mB that holds water in thirds and lava only whole; a cauldron has no block entity, so it
/// is answered by position. Powder snow is no fluid: its cauldron has no answer.
pub const FLUIDS: CapabilityKey<dyn ResourceHandler<FluidResource>> = CapabilityKey {
    name: "horizon:fluids",
    default: Some(|host, at| {
        host.fluid_tank_count(at)
            .map(|_| Box::new(GameFluids(at)) as Box<dyn ResourceHandler<FluidResource>>)
    }),
    handler: PhantomData,
};

/// The game's own [`FLUIDS`]: a mod block's tanks `.side` reaches by its table, and cauldrons.
struct GameFluids(BlockSide);
impl ResourceHandler<FluidResource> for GameFluids {
    fn size(&self, host: &mut dyn Host) -> usize {
        host.fluid_tank_count(self.0).unwrap_or(0)
    }
    fn resource(&self, host: &mut dyn Host, index: usize) -> Option<FluidResource> {
        let held = host.fluid_tank(self.0, index)?;
        Some(FluidResource::of(&held))
    }
    fn amount(&self, host: &mut dyn Host, index: usize) -> u32 {
        host.fluid_tank(self.0, index).map_or(0, |held| held.amount)
    }
    fn capacity(&self, host: &mut dyn Host, index: usize, resource: &FluidResource) -> u32 {
        host.fluid_tank_capacity(self.0, index, &resource.to_stack(1))
    }
    fn insert(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &FluidResource,
        amount: u32,
    ) -> u32 {
        let rest = host.fluid_tank_insert(self.0, index, resource.to_stack(amount), tx);
        amount - rest.map_or(0, |rest| rest.amount.min(amount))
    }
    fn extract(
        &self,
        host: &mut dyn Host,
        tx: &Transaction,
        index: usize,
        resource: &FluidResource,
        amount: u32,
    ) -> u32 {
        let held = host.fluid_tank(self.0, index);
        if !held.is_some_and(|held| resource.matches(&held)) {
            return 0;
        }
        host.fluid_tank_extract(self.0, index, amount, tx)
            .map_or(0, |out| out.amount.min(amount))
    }
}
