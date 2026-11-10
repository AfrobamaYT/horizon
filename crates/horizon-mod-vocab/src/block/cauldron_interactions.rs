//! Typed item interactions shared by cauldron authority and prediction.
use crate::{
    BlockKey, BlockState, EntityRef, Hand, Host, Identifier, ItemKey, ItemStack, ServerHost,
};
use std::sync::Arc;

/// Immutable inputs captured for one selected interaction.
#[derive(horizon_mod_api_macros::Wire, Clone, Debug)]
pub struct CauldronUse {
    /// Dimension owning the target.
    pub dimension: Identifier,
    /// Target block position.
    pub pos: [i32; 3],
    /// Complete target state before the action.
    pub state: BlockState,
    /// Acting player, scoped to the current world.
    pub actor: EntityRef,
    /// Hand used for the action.
    pub hand: Hand,
    /// Complete held stack before the action.
    pub held: ItemStack,
    /// Whether water covers the block above the target.
    pub underwater: bool,
    /// Whether creative exchange semantics apply.
    pub creative: bool,
}

/// A bounded proposal, validated against the captured local resource snapshot.
#[derive(horizon_mod_api_macros::Wire, Clone, Debug, Default)]
pub struct CauldronPlan {
    /// Optional replacement of the target block.
    pub block: Option<BlockState>,
    /// Exchange one held item for this stack using vanilla container semantics.
    pub exchange: Option<ItemStack>,
    /// Optional sound, played once on root commit.
    pub sound: Option<Identifier>,
    /// Optional fluid game event, published on root commit.
    pub fluid_event: Option<Identifier>,
    /// At most two distinct statistics: cauldron statistic and item-used statistic.
    pub statistics: Vec<Identifier>,
}

/// Selection result; a consuming result prevents subsequent normal item use.
#[derive(horizon_mod_api_macros::Wire, Clone, Debug, Default)]
pub enum CauldronDecision {
    /// Leave normal item use available.
    #[default]
    Pass,
    /// Stop item use without a plan.
    Consume,
    /// Stop item use and apply the validated plan on the server.
    Success(CauldronPlan),
}

/// A registration refused before the immutable dispatcher is published.
#[derive(horizon_mod_api_macros::Wire, Clone, Debug, PartialEq, Eq)]
pub enum CauldronRegistrationError {
    /// Both the existing and attempted owner, in that order.
    Duplicate {
        /// Existing owner followed by the attempted owner.
        owners: [String; 2],
    },
    /// The item is absent from the prepared registry.
    UnknownItem,
    /// The block is absent from the prepared registry.
    UnknownBlock,
    /// The requested dispatcher has not been declared.
    UnknownDispatcher,
    /// The callback cannot cross the active runtime boundary.
    UnsupportedCapability,
}

/// NeoForge's documented `RegisterCauldronInteractionEvent.Interaction` counterpart.
/// Uses the shared transaction and runtime limits; no cauldron-specific budget.
/// Prediction uses matching mod IDs/versions and the local resource snapshot; matching binary
/// builds and synchronized reload generations aren't established. Server correction keeps the client right.
pub trait Interaction: Send + Sync + 'static {
    /// Decide through read-only host access on either side, without authoritative writes.
    fn decide(&self, host: &dyn Host, input: &CauldronUse) -> CauldronDecision;
    /// Additional server writes run inside the existing action savepoint.
    fn fire(
        &self,
        host: &mut dyn ServerHost,
        input: &CauldronUse,
        plan: &CauldronPlan,
    ) -> CauldronFire;
}

/// Prepared dispatcher registration, corresponding to NeoForge's documented `Dispatcher` phase.
pub trait CauldronInteractions {
    /// Register an exact item entry for a block's dispatcher; vanilla entries also own their slots.
    fn register(
        &mut self,
        block: BlockKey,
        item: ItemKey,
        handler: Arc<dyn Interaction>,
    ) -> Result<(), CauldronRegistrationError>;
    /// Register over the complete prepared set, refusing the entire operation on conflict.
    fn register_to_all(
        &mut self,
        item: ItemKey,
        handler: Arc<dyn Interaction>,
    ) -> Result<(), CauldronRegistrationError>;
}

/// Authoritative callback outcome; runtime refusal and traps refuse the action.
#[derive(horizon_mod_api_macros::Wire, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CauldronFire {
    /// Permit the validated plan to commit with the callback writes.
    Applied,
    /// Abort the complete action, including nested writes and deferred effects.
    #[default]
    Refused,
}

/// Current registration owner for engine diagnostics.
pub fn cauldron_owner() -> &'static str {
    crate::content::crash::current_native_mod().unwrap_or("unknown")
}
