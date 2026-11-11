//! The published loop `block_entities.tick`: `Level.tickBlockEntities` on the server, for the block
//! entities the game ticks in one pass (hoppers, crafters, jukeboxes, beacons, creaking hearts,
//! spawners, trial spawners, vaults, beehives, furnaces, brewing stands, daylight detectors, end
//! gateways, campfires, potent sulfur and mods' ticking block entities).
//!
//! A slot sees each block entity the loop visits, in vanilla's list order, after vanilla's own
//! filters (in a chunk that ticks blocks, inside the world border, still live, not asleep):
//! [`BlockEntityTickSlot::before`] just before its tick, [`BlockEntityTickSlot::after`] just after.
//! The loop runs on the game that simulates the world (a dedicated server, single player's server
//! thread, a LAN host), never on a client, so a slot always sees server state.
//!
//! ```
//! use mod_api::loops::LoopFlow;
//! use mod_api::loops::block_entity_tick::{BlockEntityTickSlot, BlockEntityTickView, TickedKind};
//!
//! /// Hoppers wait twice as long between moves.
//! #[derive(Default)]
//! pub struct SlowHoppers;
//!
//! impl BlockEntityTickSlot for SlowHoppers {
//!     fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
//!         match view.hopper_cooldown() {
//!             Some(cooldown) if cooldown > 1 && view.game_time() % 2 == 0 => {
//!                 view.set_hopper_cooldown(cooldown - 1);
//!                 LoopFlow::Skip
//!             }
//!             _ => LoopFlow::Continue,
//!         }
//!     }
//! }
//! # let _ = TickedKind::Hopper;
//! ```

use super::{Attributed, LoopFlow, LoopMod};
use crate::{BlockStateId, DimensionKey};

/// What kind of block entity the loop is visiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum TickedKind {
    /// `HopperBlockEntity`.
    Hopper,
    /// `CrafterBlockEntity`.
    Crafter,
    /// `JukeboxBlockEntity`, while it holds a record.
    Jukebox,
    /// `BeaconBlockEntity`.
    Beacon,
    /// `CreakingHeartBlockEntity`.
    CreakingHeart,
    /// `SpawnerBlockEntity`.
    Spawner,
    /// `TrialSpawnerBlockEntity`.
    TrialSpawner,
    /// `VaultBlockEntity`.
    Vault,
    /// `BeehiveBlockEntity` (beehives and bee nests).
    Beehive,
    /// `AbstractFurnaceBlockEntity` (furnaces, blast furnaces, smokers).
    Furnace,
    /// `BrewingStandBlockEntity`.
    BrewingStand,
    /// `DaylightDetectorBlockEntity`.
    DaylightDetector,
    /// `TheEndGatewayBlockEntity`.
    EndGateway,
    /// `CampfireBlockEntity`.
    Campfire,
    /// `PotentSulfurBlockEntity`.
    PotentSulfur,
    /// A mod's ticking block entity.
    Mod,
}

/// The block entity the loop is visiting: what a slot reads, and the few fields it may write.
///
/// The game implements it on its own cursor. A slot's code is generic over it and compiled into the
/// loop, so every call inlines. Writes are typed mutators, each naming the vanilla field it sets;
/// none can panic, and one that does not apply to the visited block entity returns `false` and
/// writes nothing.
pub trait BlockEntityTickView {
    /// The block entity's position.
    fn pos(&self) -> [i32; 3];
    /// The block state at its position.
    fn state(&self) -> BlockStateId;
    /// Its kind.
    fn kind(&self) -> TickedKind;
    /// The dimension being ticked.
    fn dimension(&self) -> DimensionKey;
    /// The level's game time (`Level.getGameTime`).
    fn game_time(&self) -> i64;
    /// A hopper's transfer cooldown (`HopperBlockEntity.cooldownTime`); `None` for any other kind.
    fn hopper_cooldown(&self) -> Option<i32>;
    /// Sets a hopper's transfer cooldown (`HopperBlockEntity.setCooldown`). Any value is kept, as
    /// vanilla's field keeps any int. `false`, and no write, for any other kind.
    fn set_hopper_cooldown(&mut self, ticks: i32) -> bool;
}

/// A `[[native.loop]]` slot for `block_entities.tick`: the type a row's `with` names, which also
/// implements `Default + 'static`. Both methods default to doing nothing.
pub trait BlockEntityTickSlot {
    /// Before the visited block entity ticks. [`LoopFlow::Skip`] leaves it out this tick;
    /// [`LoopFlow::Stop`] ends the pass (what follows the loop, such as shulker box lids, still
    /// runs).
    #[inline(always)]
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        let _ = view;
        LoopFlow::Continue
    }
    /// After the visited block entity ticked; not called for one a slot skipped.
    #[inline(always)]
    fn after(&mut self, view: &mut impl BlockEntityTickView) {
        let _ = view;
    }
}

/// No mod: the game's own loop.
impl BlockEntityTickSlot for () {}

/// Two slots in order: `A` first, and `B` only while `A` answers [`LoopFlow::Continue`].
impl<A: BlockEntityTickSlot, B: BlockEntityTickSlot> BlockEntityTickSlot for (A, B) {
    #[inline(always)]
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        match self.0.before(view) {
            LoopFlow::Continue => self.1.before(view),
            answer => answer,
        }
    }
    #[inline(always)]
    fn after(&mut self, view: &mut impl BlockEntityTickView) {
        self.0.after(view);
        self.1.after(view);
    }
}

impl<N: LoopMod, T: BlockEntityTickSlot> BlockEntityTickSlot for Attributed<N, T> {
    #[inline(always)]
    fn before(&mut self, view: &mut impl BlockEntityTickView) -> LoopFlow {
        self.call(|slot| slot.before(view))
    }
    #[inline(always)]
    fn after(&mut self, view: &mut impl BlockEntityTickView) {
        self.call(|slot| slot.after(view));
    }
}
