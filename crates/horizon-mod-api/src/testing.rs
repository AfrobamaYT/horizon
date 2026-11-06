//! Test support, enabled by the `testing` feature: scoped trait-mod installation
//! ([`with_mods`]), and [`FakeHost`], a host built from small fakes so that a mod's tests run
//! without starting the game.
//!
//! ```
//! use mod_api::testing::FakeHost;
//! use mod_api::{HostTransaction, ITEMS, ItemResource, ItemStack, ServerHost, move_resources};
//!
//! let mut host = FakeHost::new();
//! let level = host.level.level();
//! host.level.block_entity([0, 64, 0]).with_items(1);
//! host.level.block_entity([1, 64, 0]).with_items(1);
//! let stone = ItemStack { item: "minecraft:stone".into(), count: 5, damage: 0, extra: None };
//! host.level.block_entity([0, 64, 0]).items.as_mut().unwrap().slots[0] = Some(stone);
//!
//! let server: &mut dyn ServerHost = &mut host;
//! let from = level.capability(server, &ITEMS, [0, 64, 0], None).unwrap();
//! let into = level.capability(server, &ITEMS, [1, 64, 0], None).unwrap();
//! let moved = server.transaction(|host, tx| {
//!     move_resources(host, &from, &into, |_: &ItemResource| true, 3, tx)
//! });
//! assert_eq!(moved.ok(), Some(3));
//! let into = host.level.block_entity_at([1, 64, 0]).unwrap().items.as_ref().unwrap();
//! assert_eq!(into.count("minecraft:stone"), 3);
//! ```
mod journal;
mod level;
mod players;
mod registries;
mod services;

pub use journal::FakeJournal;
pub use level::{FakeBattery, FakeBlockEntity, FakeContainer, FakeEntity, FakeLevel, FakeTank};
pub use players::{FakePlayer, FakePlayers};
pub use registries::FakeRegistries;
pub use services::{FakeCommands, FakeMessages};

use crate::ticket_controller::{ControllerInfo, TEST_CONTROLLERS};
use crate::transaction::{Transaction, TransactionSeal};
use crate::{
    Backend, BackendMut, BackendMutRef, BackendRef, BlockEntityRef, BlockEntityTypeKey, BlockSide,
    ClientHost, Commands, EntityRef, FluidStack, Host, Installed, ItemPlace, ItemStack, LevelRef,
    Messages, Mods, NativeWorldEpoch, PlayerRef, Players, ServerHost, TEST_MODS, Tag, Text,
    TicketController,
};

/// The world epoch of every fake's handles.
const FAKE_EPOCH: u64 = 1;

/// Run a closure with these owned mods on this thread, restoring the previous list on exit.
///
/// The test allocation is leaked because dispatch exposes a static slice. Nested scopes and
/// panic unwinding restore the preceding list; other threads are unaffected.
pub fn with_mods<R>(mods: Mods, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<&'static Installed>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_MODS.set(self.0);
        }
    }
    let controllers: &'static [_] =
        Box::leak(crate::part_table::registered_controllers(&mods).into_boxed_slice());
    let installed = Box::leak(Box::new(Installed::new(mods)));
    let _restore = Restore(TEST_MODS.replace(Some(installed)));
    let _controllers = RestoreControllers(TEST_CONTROLLERS.replace(Some(controllers)));
    f()
}

/// The ticket controller with this id that the mods of the enclosing `with_mods` registered.
pub fn ticket_controller(id: &str) -> Option<TicketController> {
    TicketController::from_id(id)
}

struct RestoreControllers(Option<&'static [ControllerInfo]>);
impl Drop for RestoreControllers {
    fn drop(&mut self) {
        TEST_CONTROLLERS.set(self.0);
    }
}

/// A server and client host for tests, composed of one fake per service: the registries, the
/// players, one level, the commands and messages services, and the transaction journal, which
/// keeps the players and the level on each transaction and puts them back when it aborts.
///
/// Set a test's world up through the fakes' fields, run the code under test with
/// `&mut host` as its `&mut dyn ServerHost`, then read the fakes back. A host method no fake
/// answers panics, naming the method, as a test double's would. The host has no client
/// backend, so client handle writes refuse.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FakeHost {
    /// The registries.
    pub registries: FakeRegistries,
    /// The players online.
    pub players: FakePlayers,
    /// The one level.
    pub level: FakeLevel,
    /// The command service.
    pub commands: FakeCommands,
    /// The message service.
    pub messages: FakeMessages,
    /// The transaction journal over the players and the level.
    pub journal: FakeJournal<(FakePlayers, FakeLevel)>,
}

impl FakeHost {
    /// A host with an empty overworld, no players and no mods loaded.
    pub fn new() -> Self {
        Self::default()
    }
    /// Player `uuid` online as entity `entity` in the level, joined if missing.
    pub fn join(&mut self, uuid: u128, entity: u32) -> &mut FakePlayer {
        self.players.join(uuid, entity, self.level.level())
    }

    fn max_stack(&self, stack: &ItemStack) -> u32 {
        self.registries.max_stack_size(&stack.item)
    }
}

impl Host for FakeHost {
    fn backend(&self) -> Option<BackendRef<'_>> {
        Some(BackendRef::of(self))
    }
    fn block_entity(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockEntityRef> {
        self.level.standing(level, pos)?;
        Some(BlockEntityRef {
            level,
            position: pos,
        })
    }
    fn level(&self, dimension: &str) -> Option<LevelRef> {
        let level = self.level.level();
        (level.dimension() == dimension).then_some(level)
    }
    fn entity(&self, id: u32) -> Option<EntityRef> {
        let known = self.players.by_entity(id).is_some() || self.level.entity_at(id).is_some();
        known.then(|| EntityRef::new(NativeWorldEpoch::new(FAKE_EPOCH), id))
    }
    fn player(&self, uuid: u128) -> Option<PlayerRef> {
        self.players.handle(uuid)
    }
    fn item_default_component(&self, item: &str, component: &str) -> Option<Tag> {
        self.registries.default_component(item, component)
    }
    fn max_stack_size(&self, item: &str) -> Option<u32> {
        Some(self.registries.max_stack_size(item))
    }
    fn is_loaded(&self, id: &str) -> bool {
        self.registries.is_loaded(id)
    }
}

impl ServerHost for FakeHost {
    fn backend_mut(&mut self) -> Option<BackendMutRef<'_>> {
        Some(BackendMutRef::of(self))
    }
    fn transaction_open(&mut self, _: TransactionSeal) {
        self.journal
            .open((self.players.clone(), self.level.clone()));
    }
    fn transaction_close(&mut self, _: TransactionSeal, commit: bool) {
        if let Some((players, level)) = self.journal.close(commit) {
            (self.players, self.level) = (players, level);
        }
    }
    fn commands(&mut self) -> &mut dyn Commands {
        &mut self.commands
    }
    fn messages(&self) -> &dyn Messages {
        &self.messages
    }
    fn players(&self) -> &dyn Players {
        &self.players
    }
}

impl ClientHost for FakeHost {}

impl Backend for FakeHost {
    fn level_in_place(&self, level: LevelRef) -> bool {
        level == self.level.level()
    }
    fn level_block(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        self.level.block_at(level, pos).map(str::to_owned)
    }
    fn block_entity_kind(&self, entity: BlockEntityRef) -> Option<BlockEntityTypeKey> {
        let standing = self.level.standing(entity.level, entity.position)?;
        standing.kind().cloned()
    }
    fn container_size(&self, at: BlockSide) -> Option<usize> {
        let items = self.level.standing(at.level, at.pos)?.items.as_ref()?;
        Some(items.slots.len())
    }
    fn container_stack(&self, at: BlockSide, slot: usize) -> Option<ItemStack> {
        let items = self.level.standing(at.level, at.pos)?.items.as_ref()?;
        items.slots.get(slot)?.clone()
    }
    fn container_capacity(&self, at: BlockSide, slot: usize, stack: &ItemStack) -> u32 {
        let items = self
            .level
            .standing(at.level, at.pos)
            .and_then(|standing| standing.items.as_ref());
        items.map_or(0, |items| items.capacity(slot, self.max_stack(stack)))
    }
    fn fluid_tank_count(&self, at: BlockSide) -> Option<usize> {
        Some(self.level.standing(at.level, at.pos)?.tanks.len())
    }
    fn fluid_tank(&self, at: BlockSide, tank: usize) -> Option<FluidStack> {
        let standing = self.level.standing(at.level, at.pos)?;
        standing.tanks.get(tank)?.held.clone()
    }
    fn fluid_tank_capacity(&self, at: BlockSide, tank: usize, _: &FluidStack) -> u32 {
        let standing = self.level.standing(at.level, at.pos);
        standing
            .and_then(|standing| standing.tanks.get(tank))
            .map_or(0, |tank| tank.capacity)
    }
    fn energy_amount(&self, at: BlockSide) -> Option<u32> {
        let energy = self.level.standing(at.level, at.pos)?.energy.as_ref()?;
        Some(energy.amount)
    }
    fn energy_capacity(&self, at: BlockSide) -> u32 {
        let standing = self.level.standing(at.level, at.pos);
        standing
            .and_then(|standing| standing.energy.as_ref())
            .map_or(0, |energy| energy.capacity)
    }
    fn entity_kind(&self, entity: EntityRef) -> Option<String> {
        if self.players.by_entity(entity.id()).is_some() {
            return Some("minecraft:player".to_owned());
        }
        Some(self.level.entity_at(entity.id())?.kind.clone())
    }
    fn entity_level(&self, entity: EntityRef) -> Option<LevelRef> {
        if let Some((_, player)) = self.players.by_entity(entity.id()) {
            return Some(player.level);
        }
        Some(self.level.entity_at(entity.id())?.level)
    }
    fn entity_container_size(&self, _: u32) -> Option<usize> {
        None
    }
    fn entity_automation_size(&self, _: u32) -> Option<usize> {
        None
    }
    fn player_selected_slot(&self, player: PlayerRef) -> Option<usize> {
        Some(self.players.get(player.uuid())?.selected_slot)
    }
    fn player_name(&self, player: PlayerRef) -> Option<String> {
        Some(self.players.get(player.uuid())?.name.clone())
    }
    fn item_container_slots(&self, stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
        self.registries.container_slots(stack)
    }
}

impl BackendMut for FakeHost {
    fn container_insert(
        &mut self,
        at: BlockSide,
        slot: usize,
        stack: ItemStack,
        _: &Transaction,
    ) -> Option<ItemStack> {
        let max_stack = self.max_stack(&stack);
        let standing = self.level.standing_mut(at.level, at.pos);
        match standing.and_then(|standing| standing.items.as_mut()) {
            Some(items) => items.insert(slot, stack, max_stack),
            None => Some(stack),
        }
    }
    fn container_extract(
        &mut self,
        at: BlockSide,
        slot: usize,
        max: u32,
        _: &Transaction,
    ) -> Option<ItemStack> {
        let standing = self.level.standing_mut(at.level, at.pos)?;
        standing.items.as_mut()?.extract(slot, max)
    }
    fn fluid_tank_insert(
        &mut self,
        at: BlockSide,
        tank: usize,
        fluid: FluidStack,
        _: &Transaction,
    ) -> Option<FluidStack> {
        let standing = self.level.standing_mut(at.level, at.pos);
        match standing.and_then(|standing| standing.tanks.get_mut(tank)) {
            Some(tank) => tank.insert(fluid),
            None => Some(fluid),
        }
    }
    fn fluid_tank_extract(
        &mut self,
        at: BlockSide,
        tank: usize,
        max: u32,
        _: &Transaction,
    ) -> Option<FluidStack> {
        let standing = self.level.standing_mut(at.level, at.pos)?;
        standing.tanks.get_mut(tank)?.extract(max)
    }
    fn energy_insert(&mut self, at: BlockSide, amount: u32, _: &Transaction) -> u32 {
        let standing = self.level.standing_mut(at.level, at.pos);
        standing
            .and_then(|standing| standing.energy.as_mut())
            .map_or(0, |energy| energy.insert(amount))
    }
    fn energy_extract(&mut self, at: BlockSide, amount: u32, _: &Transaction) -> u32 {
        let standing = self.level.standing_mut(at.level, at.pos);
        standing
            .and_then(|standing| standing.energy.as_mut())
            .map_or(0, |energy| energy.extract(amount))
    }
    fn entity_uuid(&mut self, entity: EntityRef) -> Option<u128> {
        if let Some((uuid, _)) = self.players.by_entity(entity.id()) {
            return Some(uuid);
        }
        Some(self.level.entity_at(entity.id())?.uuid)
    }
    fn game_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        let (player, slot) = self.players.place(place)?;
        player.slots.get(&slot).cloned()
    }
    fn game_insert(&mut self, place: &ItemPlace, stack: ItemStack, _: &Transaction) -> u32 {
        let max_stack = self.max_stack(&stack);
        match self.players.place(place) {
            Some((player, slot)) if stack.count > 0 => player.insert(slot, stack, max_stack),
            _ => 0,
        }
    }
    fn game_extract(&mut self, place: &ItemPlace, stack: ItemStack, _: &Transaction) -> u32 {
        self.players
            .place(place)
            .map_or(0, |(player, slot)| player.extract(slot, &stack))
    }
    fn player_action_bar(&mut self, player: PlayerRef, line: Text) -> bool {
        let Some(player) = self.players.get_mut(player.uuid()) else {
            return false;
        };
        player.action_bar.push(line);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FLUIDS, FluidResource, Hand, HostTransaction, ItemResource, move_resources};

    fn water(amount: u32) -> FluidStack {
        FluidStack::new("minecraft:water", amount)
    }

    #[test]
    fn a_fluid_moves_between_tanks_of_the_same_fluid_and_an_abort_puts_both_back() {
        let mut host = FakeHost::new();
        let level = host.level.level();
        host.level.block_entity([0, 64, 0]).tanks = vec![FakeTank {
            held: Some(water(800)),
            capacity: 1000,
        }];
        host.level.block_entity([1, 64, 0]).tanks = vec![FakeTank {
            held: Some(water(700)),
            capacity: 1000,
        }];
        host.level.block_entity([2, 64, 0]).tanks = vec![FakeTank {
            held: Some(FluidStack::new("minecraft:lava", 100)),
            capacity: 1000,
        }];
        let server: &mut dyn ServerHost = &mut host;
        let tank = |host: &mut dyn ServerHost, x| {
            level.capability(host, &FLUIDS, [x, 64, 0], None).unwrap()
        };
        let (from, into, lava) = (tank(server, 0), tank(server, 1), tank(server, 2));
        let any = |_: &FluidResource| true;
        let aborted = server.transaction(|host, tx| {
            let moved = move_resources(host, &from, &into, any, 1000, tx);
            Err::<(), _>(moved)
        });
        assert_eq!(aborted, Err(Ok(300)));
        // Lava takes no water; water fills only up to the target's room.
        let refused =
            server.transaction(|host, tx| move_resources(host, &from, &lava, any, 1000, tx));
        assert_eq!(refused, Ok(0));
        let moved =
            server.transaction(|host, tx| move_resources(host, &from, &into, any, 1000, tx));
        assert_eq!(moved, Ok(300));
        let amount = |x| host.level.block_entity_at([x, 64, 0]).unwrap().tanks[0].amount();
        assert_eq!([amount(0), amount(1), amount(2)], [500, 1000, 100]);
    }

    #[test]
    fn what_a_players_hand_cannot_hold_is_given_to_them_and_an_abort_takes_it_back() {
        let mut host = FakeHost::new();
        host.join(7, 41).selected_slot = 2;
        host.registries
            .item_component("minecraft:egg", "minecraft:max_stack_size", Tag::Int(16));
        let hand = ItemPlace::PlayerHand {
            player: 7,
            hand: Hand::Main,
        };
        let eggs = ItemResource::of(&ItemStack {
            item: "minecraft:egg".into(),
            count: 1,
            damage: 0,
            extra: None,
        });
        let server: &mut dyn ServerHost = &mut host;
        let aborted = server.transaction(|host, tx| Err::<(), _>(hand.insert(host, tx, &eggs, 20)));
        assert_eq!(aborted, Err(20));
        assert!(host.players.get(7).unwrap().slots.is_empty());
        let server: &mut dyn ServerHost = &mut host;
        let went = server.transaction(|host, tx| Ok::<_, ()>(hand.insert(host, tx, &eggs, 20)));
        assert_eq!(went, Ok(20));
        let player = host.players.get(7).unwrap();
        assert_eq!(player.main_hand().map(|held| held.count), Some(16));
        assert_eq!(player.slots.keys().collect::<Vec<_>>(), [&2]);
        let given: Vec<u32> = player.given.iter().map(|stack| stack.count).collect();
        assert_eq!(given, [4]);
    }
}
