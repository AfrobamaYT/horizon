//! `AbstractContainerMenu.clicked` over vanilla's slot layouts: every
//! `ContainerInput` (pickup, quick move, swap, clone, throw, the three quick
//! craft drags, pickup all) for the player inventory, the crafting table, the
//! chest family, hoppers, dispensers, shulker boxes, the furnaces, the brewing
//! stand, the crafter (its client menu and the block entity's server menu with
//! `setSlotState`), the loom (`clickMenuButton`, `slotsChanged`), the
//! stonecutter (its recipe buttons and result) and the merchant (`MerchantMenu` over a `merchant::Session`: payment slots, the
//! result slot's trade, `tryMoveItems`).
//! `tools/test_menu_click_parity.py` compares every click with vanilla's menu
//! classes.
use crate::{
    banner::{self, Banners},
    brewing::Brewing,
    crafting::Recipes,
    items::{ItemStack, Items},
};
#[cfg(test)]
use horizon_world::native_load_context::{DataLoadOrigin, LoadContext, LoadInputs};
use horizon_world::native_load_context::{LoopContext, LoopSide};
use mod_api::__private::game::IngredientTest;
pub use mod_api::menu::{
    AnvilCraft, AnvilUpdate, AnvilVanilla, CarriedAccess, GrindstonePlace, GrindstoneTake,
    LogicalSide, MenuButtonContext, SlotView, StackedOnOther,
};
use std::sync::Arc;

mod bundle;
pub mod place;

/// `ServerPlayer.getRecipeBook`: whether the player knows a recipe.
pub trait KnownRecipes {
    fn knows(&self, recipe_id: &str) -> bool;
}

impl KnownRecipes for crate::recipe_book::Settings {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.recipes.contains(&q) || self.recipes.contains(recipe_id)
    }
}

impl KnownRecipes for std::collections::BTreeSet<String> {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.contains(&q) || self.contains(recipe_id)
    }
}

impl KnownRecipes for std::collections::HashSet<String> {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.contains(&q) || self.contains(recipe_id)
    }
}

impl KnownRecipes for [String] {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.iter().any(|r| r == &q || r == recipe_id)
    }
}

impl KnownRecipes for Vec<String> {
    fn knows(&self, recipe_id: &str) -> bool {
        self.as_slice().knows(recipe_id)
    }
}

impl<const N: usize> KnownRecipes for [&str; N] {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.iter().any(|r| *r == q.as_str() || *r == recipe_id)
    }
}

impl KnownRecipes for &[&str] {
    fn knows(&self, recipe_id: &str) -> bool {
        let q = crate::crafting::qualified(recipe_id);
        self.iter().any(|r| *r == q.as_str() || *r == recipe_id)
    }
}

/// What a menu needs to know about items and recipes.
pub struct Env<'a> {
    /// UUID of the player operating this menu, when a player is available.
    pub player: Option<u128>,
    pub player_entity_id: Option<i32>,
    /// The side and settings of the resource generation this menu runs in: the
    /// one source of its side, and what a hook inside a menu loop is given.
    pub loop_context: &'a LoopContext,
    pub items: &'a Items,
    pub recipes: &'a Recipes,
    pub brewing: &'a Brewing,
    pub banners: &'a Banners,
    /// The stonecutter's recipes.
    pub station_recipes: &'a crate::stations::StationRecipes,
    /// The grindstone's and the anvil's enchantment rules.
    pub enchantments: &'a crate::enchant::Enchantments,
    /// Item display names: the anvil compares the rename box with the
    /// input's hover name.
    pub names: &'a crate::creative::Names,
    /// What a merchant menu needs for offers made while it is open (level-ups).
    pub trade: Option<&'a crate::merchant::TradeEnv<'a>>,
    /// The level's maps (`ContainerLevelAccess`): the cartography table reads
    /// and makes them. None is a client's menu (`ContainerLevelAccess.NULL`):
    /// it makes no result, and a taken result keeps its map id.
    pub maps: Option<&'a dyn Maps>,
    /// `GameRules.LIMITED_CRAFTING`: when true, only known or special recipes craft.
    pub limited_crafting: bool,
    /// The recipes the player knows (`ServerPlayer.getRecipeBook().contains(id)`).
    pub known_recipes: Option<&'a dyn KnownRecipes>,
}

/// The maps of the level a menu is in.
pub trait Maps {
    /// `MapItem.getSavedData`: whether map `id` is locked, and its scale;
    /// None when the level has no such map.
    fn state(&self, id: i32) -> Option<(bool, u8)>;
    /// `MapItem.scaleMap` (`scale`) or `lockMap`: the copy of map `id` the
    /// level makes under a new id (`getFreeMapId`), None without the map.
    fn derive(&self, id: i32, scale: bool) -> Option<i32>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Input {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}

impl Input {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "PICKUP" => Self::Pickup,
            "QUICK_MOVE" => Self::QuickMove,
            "SWAP" => Self::Swap,
            "CLONE" => Self::Clone,
            "THROW" => Self::Throw,
            "QUICK_CRAFT" => Self::QuickCraft,
            "PICKUP_ALL" => Self::PickupAll,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Inventory,
    Crafting,
    Chest(usize),
    Hopper,
    Dispenser,
    Shulker,
    /// The cooking recipe type: smelting, blasting or smoking.
    Furnace(&'static str),
    Brewing,
    /// `CrafterMenu` over a plain crafting container (the client's menu).
    Crafter,
    /// `CrafterMenu` over the `CrafterBlockEntity` (the server's menu): the
    /// result follows the grid, and filling a disabled slot enables it.
    CrafterBlock,
    Loom,
    /// `StonecutterMenu`: the input, the result, then the inventory.
    Stonecutter,
    /// `GrindstoneMenu`: two inputs, the result, then the inventory.
    Grindstone,
    /// `AnvilMenu`: two inputs, the result, then the inventory.
    Anvil,
    /// `SmithingMenu`: template, base, addition, the result, then the inventory.
    Smithing,
    /// `EnchantmentMenu`: the item, the lapis, then the inventory; its data
    /// slots are the three costs, the seed and the clues.
    Enchanting,
    /// `BeaconMenu`: the payment slot, then the inventory; its data slots are
    /// the pyramid's levels and the two powers (`encodeEffect`: id + 1).
    Beacon,
    /// `CartographyTableMenu`: the map, the paper / map / glass pane, the
    /// result, then the inventory.
    Cartography,
    /// `LecternMenu` over the `LecternBlockEntity`: the book's one slot (no
    /// inventory slots); its data slot is the open page.
    Lectern,
    /// `MerchantMenu`: two payment slots, the result, then the inventory.
    Merchant,
    /// `HorseInventoryMenu`: the saddle and body armour slots (`ArmorSlot`s
    /// of the mount, whose entity type is `Menu::mount`), a chest of three
    /// rows of this many columns (`getInventoryColumns`), then the inventory.
    Mount(usize),
    /// `CreativeModeInventoryScreen.ItemPickerMenu`: an item tab's 45
    /// display slots, then the hotbar.
    ItemPicker,
    /// A content mod's menu type: its slots, then the inventory.
    Mod(&'static crate::containers::ModMenuType),
}

impl Kind {
    pub fn parse(name: &str) -> Option<Self> {
        if let Some(rows) = name.strip_prefix("chest") {
            return rows.parse().ok().map(Self::Chest);
        }
        if let Some(columns) = name.strip_prefix("mount") {
            return columns.parse().ok().map(Self::Mount);
        }
        Some(match name {
            "inventory" => Self::Inventory,
            "crafting" => Self::Crafting,
            "hopper" => Self::Hopper,
            "dispenser" => Self::Dispenser,
            "shulker" => Self::Shulker,
            "furnace" => Self::Furnace("smelting"),
            "blast_furnace" => Self::Furnace("blasting"),
            "smoker" => Self::Furnace("smoking"),
            "brewing" => Self::Brewing,
            "crafter" => Self::Crafter,
            "crafter_be" => Self::CrafterBlock,
            "loom" => Self::Loom,
            "stonecutter" => Self::Stonecutter,
            "grindstone" => Self::Grindstone,
            "anvil" => Self::Anvil,
            "smithing" => Self::Smithing,
            "enchanting" => Self::Enchanting,
            "beacon" => Self::Beacon,
            "cartography" => Self::Cartography,
            "lectern" => Self::Lectern,
            "merchant" => Self::Merchant,
            "item_picker" => Self::ItemPicker,
            _ => return None,
        })
    }
}

/// `Slot` subclasses that change the rules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Plain,
    /// `ArmorSlot` for the equipment slot with this `EquipmentSlot.getIndex`.
    Armor(usize),
    Shulker,
    Fuel,
    /// `FurnaceResultSlot`.
    Output,
    /// `ResultSlot` of a crafting grid.
    CraftResult,
    /// `CrafterSlot`: refuses items while its disabled flag (data slot) is set.
    CrafterIn,
    /// `NonInteractiveResultSlot`.
    Inert,
    /// `BrewingStandMenu.PotionSlot`.
    Potion,
    /// `BrewingStandMenu.IngredientsSlot`.
    Ingredient,
    /// `BrewingStandMenu.FuelSlot`.
    BrewFuel,
    /// The loom's banner, dye and pattern slots.
    LoomBanner,
    LoomDye,
    LoomPattern,
    /// The loom's result slot (a plain slot whose `onTake` uses the inputs).
    LoomResult,
    /// `MerchantResultSlot`.
    MerchantResult,
    /// The stonecutter's result slot (`onTake` uses up one input).
    StoneResult,
    /// The grindstone's inputs: damageable or enchanted items.
    GrindInput,
    /// The grindstone's result slot (`onTake` clears both inputs).
    GrindResult,
    /// A smithing table input: its recipe property set (template, base, addition).
    SmithInput(usize),
    /// `ItemCombinerMenu`'s result slot (the anvil's and the smithing table's).
    CombineResult,
    /// The enchanting table's item slot (one item).
    EnchantItem,
    /// The enchanting table's lapis slot.
    Lapis,
    /// `BeaconMenu.PaymentSlot`: one beacon payment item.
    BeaconPay,
    /// The cartography table's map slot (anything with a map id).
    CartoMap,
    /// Its paper, empty map or glass pane slot.
    CartoExtra,
    /// Its result (`onTake`: one of each input, the map post-processed).
    CartoResult,
    /// `CustomCreativeSlot`: a creative tab's display slot, which the
    /// screen clicks itself; drags and PICKUP_ALL pass it by.
    CreativeDisplay,
    /// An `ArmorSlot` of a mount for this `EquipmentSlot` ("saddle" or
    /// "body"): `isEquippableInSlot` for `Menu::mount`, one item.
    MountEquip(&'static str),
    /// A mod menu type's slot.
    Mod,
    /// A mod menu type's result (`results`): players only take from it.
    ModResult,
    /// A slot the mod's screen leaves out: no click reaches it.
    Hidden,
}

#[derive(Clone, Copy)]
struct Slot {
    container: usize,
    index: usize,
    role: Role,
}

struct MenuSlotView<'a, 'e> {
    menu: &'a mut Menu,
    env: &'a Env<'e>,
    slot: usize,
}

impl SlotView for MenuSlotView<'_, '_> {
    fn item(&self) -> Option<ItemStack> {
        self.menu.slot(self.slot).cloned()
    }
    fn may_place(&self, stack: &ItemStack) -> bool {
        self.menu.may_place(self.env, self.slot, stack)
    }
    fn may_pickup(&self) -> bool {
        self.menu.may_pickup(self.env, self.slot)
    }
    fn allow_modification(&self) -> bool {
        if self.menu.slot(self.slot).is_some() {
            self.menu.allow_modification(self.env, self.slot)
        } else {
            let empty = ItemStack::new("minecraft:air", 0);
            self.may_pickup() && self.may_place(&empty)
        }
    }
    fn safe_insert(&mut self, mut stack: Option<ItemStack>, count: u32) -> Option<ItemStack> {
        self.menu
            .safe_insert(self.env, self.slot, &mut stack, count);
        stack
    }
    fn safe_take(&mut self, count: u32, max: u32) -> Option<ItemStack> {
        self.menu.safe_take(self.env, self.slot, count, max)
    }
}

/// NeoForge's `ItemStackedOnOtherEvent`, before either item override.
#[inline(never)]
pub fn native_item_stacked_on_other(
    context: &LoopContext,
    event: &mut StackedOnOther<'_>,
    canceled: &mut bool,
) {
    crate::native_client_hooks::item_stacked_on_other(context, event, canceled);
}

/// `BeaconBlockEntity.BEACON_EFFECTS`: the powers of each pyramid level.
pub const BEACON_EFFECTS: [&[&str]; 4] = [
    &["minecraft:speed", "minecraft:haste"],
    &["minecraft:resistance", "minecraft:jump_boost"],
    &["minecraft:strength"],
    &["minecraft:regeneration"],
];

/// `BeaconBlockEntity.getRequiredLevelsFor`: 0 for no power, `i32::MAX`
/// for one no level gives.
pub fn beacon_required_levels(effect: Option<&str>) -> i32 {
    let Some(effect) = effect else {
        return 0;
    };
    BEACON_EFFECTS
        .iter()
        .position(|tier| tier.contains(&effect))
        .map_or(i32::MAX, |i| i as i32 + 1)
}

/// `Item.canFitInsideContainerItems` (`ShulkerBoxSlot.mayPlace`): shulker
/// boxes cannot nest.
pub fn fits_inside_container_items(stack: &ItemStack) -> bool {
    !stack.item.ends_with("shulker_box")
}

/// The containers a menu spans.
pub(crate) const PLAYER: usize = 0;
pub(crate) const MAIN: usize = 1;
const GRID: usize = 2;
const RESULT: usize = 3;
/// `Container.getMaxStackSize`.
const CONTAINER_MAX: u32 = 99;
const OFFHAND: usize = 40;

pub struct Menu {
    kind: Kind,
    registry_id: Option<Box<str>>,
    slots: Vec<Slot>,
    containers: [Vec<Option<ItemStack>>; 4],
    grid_size: usize,
    pub carried: Option<ItemStack>,
    pub data: Vec<i32>,
    pub creative: bool,
    /// The selected hotbar slot (`Inventory.selected`).
    pub selected: usize,
    /// Dropped stacks (canonical text), `xp+N` and the like, in order.
    pub drops: Vec<String>,
    /// The dropped stacks themselves, for the caller to throw.
    pub dropped: Vec<ItemStack>,
    quick_status: i32,
    quick_type: i32,
    quick_slots: Vec<usize>,
    /// `LoomMenu.selectablePatterns`.
    pub loom_patterns: Vec<Arc<str>>,
    /// Sounds the menu plays (`ui.loom.take_result`), for the caller.
    pub sounds: Vec<&'static str>,
    /// Sounds the player makes (a bundle's insert and remove), for the
    /// caller to play at the player as `Entity.playSound`.
    pub player_sounds: Vec<&'static str>,
    /// Recipes whose result was taken (`awardUsedRecipes`) and the stacks that
    /// went into each, for the caller.
    pub crafted: Vec<(String, Vec<Option<ItemStack>>)>,
    /// A crafting grid's result taken (`ResultSlot.onTake`): the recipe and
    /// the stack taken, for the caller (`native_item_crafted`).
    pub item_crafted: Vec<(String, ItemStack)>,
    /// `ItemStack.onCraftedBy` counts from merchant result callbacks, without a recipe id.
    pub crafted_counts: Vec<(Arc<str>, i32)>,
    /// `MerchantResultSlot.removeCount`, reset by checkTakeAchievements.
    merchant_remove_count: i32,
    /// Potions taken from a brewing stand's bottle slots
    /// (`PotionSlot.onTake`), for the caller.
    pub brewed: Vec<ItemStack>,
    /// Set by a server caller when a mod listens: every stack taken from a
    /// brewing stand's bottle slots then goes to `brewed_potion_takes`.
    pub brewed_potion_hook: bool,
    /// Stacks taken from brewing stand bottle slots, for
    /// `PlayerBrewedPotionEvent`.
    pub brewed_potion_takes: Vec<ItemStack>,
    /// Set by a server caller when a mod listens: every
    /// `FurnaceResultSlot.checkTakeAchievements` then goes to `item_smelted`.
    pub item_smelted_hook: bool,
    /// What each `checkTakeAchievements` of a furnace's result slot passed,
    /// for `ItemSmeltedEvent`: the stack (`None` for vanilla's empty one)
    /// and `removeCount`.
    pub item_smelted: Vec<(Option<ItemStack>, u32)>,
    /// Set by a server caller when a mod listens: every smithing result
    /// taken then goes to `item_smithed`.
    pub item_smithed_hook: bool,
    /// Each smithing result taken (`SmithingMenu.onTake`, or `onQuickCraft`
    /// for a shift-click), for `ItemSmithedEvent`: the template, base and
    /// addition as they were before the take used them, and the stack taken.
    pub item_smithed: Vec<([Option<ItemStack>; 3], ItemStack)>,
    /// `FurnaceResultSlot.removeCount`.
    output_removed: u32,
    /// `LivingEntity.onEquipItem` calls that change the item: the
    /// `EquipmentSlot` name and the new stack, in order, for the caller.
    pub equipped: Vec<(&'static str, Option<ItemStack>)>,
    /// The merchant a merchant menu trades with.
    pub merchant: Option<Box<crate::merchant::Session>>,
    /// The stonecutter's recipe list (`recipesForInput`).
    pub stone: Stonecutter,
    /// The grindstone result's `getExperienceAmount` sum before its roll,
    /// set when the result was taken: the caller (with a level) awards the
    /// orbs and plays level event 1042.
    pub ground: Option<i32>,
    /// `Player.experienceLevel`, which the anvil's result needs.
    pub levels: i32,
    /// The levels the menu gave the player (`giveExperienceLevels`), for the caller.
    pub level_change: i32,
    /// `AnvilMenu`'s state besides its slots and its cost data slot.
    pub anvil: Anvil,
    /// The anvil's or the smithing table's result was taken: the caller (with
    /// a level) runs the rest of its `onTake` (the anvil's wear and level
    /// event 1030 or 1029, the smithing table's 1044).
    pub combined: bool,
    /// The caller fires `AnvilCraftEvent.Pre` for a taken anvil result: its
    /// `onTake` waits in `anvil_craft` for the caller to run `anvil_take`, or
    /// to refuse the take with `anvil_cancel` (from an `anvil_undo` made
    /// before the click).
    pub anvil_hook: bool,
    /// The anvil result taken while `anvil_hook` is set, as it was before
    /// crafting.
    pub anvil_craft: Option<AnvilCraft>,
    /// The caller fires `GrindstoneEvent.OnTakeItem` for a taken grindstone
    /// result: its `onTake` waits in `grind_take` for `grindstone_taken`.
    pub grind_hook: bool,
    /// The grindstone result taken while `grind_hook` is set, with its
    /// inputs as they were.
    pub grind_take: Option<GrindstoneTake>,
    /// `GrindstoneEvent.OnPlaceItem`'s experience for the result now in the
    /// slot, or -1 for vanilla's.
    grind_xp: i32,
    /// `AnvilMenu.createResult` ran since `anvil_created` last asked.
    anvil_made: bool,
    /// The enchanting table's item changed to an enchantable one: the caller
    /// (with a level) fills the costs and clues (`slotsChanged`'s
    /// `access.execute`).
    pub enchant_changed: bool,
    /// An enchanting button the menu took (`clickMenuButton` passed its
    /// checks): the caller (with a level) enchants.
    pub enchant_button: Option<usize>,
    /// `Player.mayBuild` (not adventure mode): the lectern's Take Book.
    pub may_build: bool,
    /// A mount menu's entity type (`minecraft:horse`): what its saddle and
    /// body slots take.
    pub mount: Option<String>,
}

/// `AnvilMenu`'s state besides its slots and `cost`.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Anvil {
    /// `itemName`: the rename box's text (`ServerboundRenameItemPacket`).
    pub name: Option<String>,
    /// `repairItemCountCost`.
    pub repair_count: u32,
    /// `onlyRenaming`.
    pub only_renaming: bool,
}

/// An anvil menu before a click, for `Menu::anvil_cancel`: the anvil's three
/// slots and cost, the player's slots and the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct AnvilUndo {
    inputs: Vec<Option<ItemStack>>,
    result: Option<ItemStack>,
    cost: i32,
    player: Vec<Option<ItemStack>>,
    carried: Option<ItemStack>,
}

/// `StonecutterMenu`'s state besides its slots and `selectedRecipeIndex`.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Stonecutter {
    /// `recipesForInput`: indices into `StationRecipes.stonecutting`.
    pub recipes: Vec<usize>,
    /// The item the list was made for (`input`; None for nothing).
    pub input: Option<Arc<str>>,
    /// `ResultContainer.recipeUsed`.
    pub used: Option<usize>,
}

fn same_item(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item
}

fn split(stack: &mut ItemStack, amount: u32) -> ItemStack {
    let real = amount.min(stack.count);
    let mut out = stack.clone();
    out.count = real;
    stack.count -= real;
    out
}

/// `LecternBlockEntity.getPageCount`: a written book's pages, else a
/// writable book's.
pub fn book_pages(stack: &ItemStack) -> usize {
    stack.data().map_or(0, |d| match &d.written {
        Some(w) => w.pages.len(),
        None => d.writable_pages.len(),
    })
}

fn tidy(slot: &mut Option<ItemStack>) {
    if slot.as_ref().is_some_and(|s| s.count == 0) {
        *slot = None;
    }
}

/// Dispatch registered vanilla and mod menu slot placement and maximum stack rules.
/// The slot is its backing container index. The shared application bounds the
/// final maximum.
#[inline(always)]
pub fn native_menu_slot_rules(
    context: &LoopContext,
    menu: &str,
    slot: usize,
    stack: &ItemStack,
    may_place: &mut bool,
    max_stack: &mut u32,
) {
    crate::native_client_hooks::menu_slot_rules(context, menu, slot, stack, may_place, max_stack);
}

/// Apply the scoped rule to a slot's own answers, then enforce its item cap.
pub fn apply_menu_slot_rules(
    context: &LoopContext,
    menu: &str,
    slot: usize,
    stack: &ItemStack,
    mut may_place: bool,
    mut max_stack: u32,
    item_max: u32,
) -> (bool, u32) {
    native_menu_slot_rules(context, menu, slot, stack, &mut may_place, &mut max_stack);
    (may_place, max_stack.clamp(1, CONTAINER_MAX).min(item_max))
}

/// Vanilla quick-craft count for one slot, based on the original carried stack.
pub(crate) fn quick_craft_count(
    kind: u8,
    source: u32,
    slots: usize,
    have: u32,
    max: u32,
    item_max: u32,
) -> u32 {
    let each = match kind {
        0 => (source as f32 / slots as f32).floor() as u32,
        1 => 1,
        2 => item_max,
        _ => source,
    };
    (each + have).min(max)
}

/// An index outside the slot list: vanilla's `IndexOutOfBoundsException`.
pub const OUT_OF_BOUNDS: &str = "IndexOutOfBoundsException";

impl Menu {
    pub fn new(kind: Kind) -> Self {
        let mut containers: [Vec<Option<ItemStack>>; 4] = Default::default();
        containers[PLAYER] = vec![None; 41];
        let mut slots = Vec::new();
        let mut grid_size = 0;
        let main = |n: usize, role: Role| -> Vec<Slot> {
            (0..n)
                .map(|index| Slot {
                    container: MAIN,
                    index,
                    role,
                })
                .collect()
        };
        let player = |slots: &mut Vec<Slot>| {
            for i in (9..36).chain(0..9) {
                slots.push(Slot {
                    container: PLAYER,
                    index: i,
                    role: Role::Plain,
                });
            }
        };
        let grid = |slots: &mut Vec<Slot>, n: usize| {
            slots.push(Slot {
                container: RESULT,
                index: 0,
                role: Role::CraftResult,
            });
            for index in 0..n * n {
                slots.push(Slot {
                    container: GRID,
                    index,
                    role: Role::Plain,
                });
            }
        };
        let mut data = 0;
        match kind {
            Kind::Inventory => {
                grid_size = 2;
                grid(&mut slots, 2);
                for (i, eq) in [3usize, 2, 1, 0].into_iter().enumerate() {
                    slots.push(Slot {
                        container: PLAYER,
                        index: 39 - i,
                        role: Role::Armor(eq),
                    });
                }
                player(&mut slots);
                slots.push(Slot {
                    container: PLAYER,
                    index: OFFHAND,
                    role: Role::Plain,
                });
            }
            Kind::Crafting => {
                grid_size = 3;
                grid(&mut slots, 3);
                player(&mut slots);
            }
            Kind::Chest(rows) => {
                containers[MAIN] = vec![None; rows * 9];
                slots.extend(main(rows * 9, Role::Plain));
                player(&mut slots);
            }
            Kind::Hopper => {
                containers[MAIN] = vec![None; 5];
                slots.extend(main(5, Role::Plain));
                player(&mut slots);
            }
            Kind::Dispenser => {
                containers[MAIN] = vec![None; 9];
                slots.extend(main(9, Role::Plain));
                player(&mut slots);
            }
            Kind::Shulker => {
                containers[MAIN] = vec![None; 27];
                slots.extend(main(27, Role::Shulker));
                player(&mut slots);
            }
            Kind::Mod(t) => {
                containers[MAIN] = vec![None; t.slots];
                slots.extend((0..t.slots).map(|index| Slot {
                    container: MAIN,
                    index,
                    role: match t.results.contains(&index) {
                        true => Role::ModResult,
                        false => Role::Mod,
                    },
                }));
                player(&mut slots);
            }
            Kind::Crafter | Kind::CrafterBlock => {
                containers[MAIN] = vec![None; 9];
                slots.extend(main(9, Role::CrafterIn));
                player(&mut slots);
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::Inert,
                });
                data = 10;
            }
            Kind::Loom => {
                containers[MAIN] = vec![None; 3];
                for (index, role) in [Role::LoomBanner, Role::LoomDye, Role::LoomPattern]
                    .into_iter()
                    .enumerate()
                {
                    slots.push(Slot {
                        container: MAIN,
                        index,
                        role,
                    });
                }
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::LoomResult,
                });
                player(&mut slots);
                data = 1;
            }
            Kind::Stonecutter => {
                containers[MAIN] = vec![None; 1];
                slots.extend(main(1, Role::Plain));
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::StoneResult,
                });
                player(&mut slots);
                // selectedRecipeIndex: DataSlot.standalone() starts at 0.
                data = 1;
            }
            Kind::Grindstone => {
                containers[MAIN] = vec![None; 2];
                slots.extend(main(2, Role::GrindInput));
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::GrindResult,
                });
                player(&mut slots);
            }
            Kind::Anvil => {
                containers[MAIN] = vec![None; 2];
                slots.extend(main(2, Role::Plain));
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::CombineResult,
                });
                player(&mut slots);
                // cost: DataSlot.standalone() starts at 0.
                data = 1;
            }
            Kind::Smithing => {
                containers[MAIN] = vec![None; 3];
                slots.extend((0..3).map(|index| Slot {
                    container: MAIN,
                    index,
                    role: Role::SmithInput(index),
                }));
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::CombineResult,
                });
                player(&mut slots);
                // hasRecipeError.
                data = 1;
            }
            Kind::Enchanting => {
                containers[MAIN] = vec![None; 2];
                slots.push(Slot {
                    container: MAIN,
                    index: 0,
                    role: Role::EnchantItem,
                });
                slots.push(Slot {
                    container: MAIN,
                    index: 1,
                    role: Role::Lapis,
                });
                player(&mut slots);
                // costs[3], enchantmentSeed, enchantClue[3], levelClue[3].
                data = 10;
            }
            Kind::Beacon => {
                containers[MAIN] = vec![None; 1];
                slots.extend(main(1, Role::BeaconPay));
                player(&mut slots);
                // levels, primary, secondary.
                data = 3;
            }
            Kind::Cartography => {
                containers[MAIN] = vec![None; 2];
                for (index, role) in [Role::CartoMap, Role::CartoExtra].into_iter().enumerate() {
                    slots.push(Slot {
                        container: MAIN,
                        index,
                        role,
                    });
                }
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::CartoResult,
                });
                player(&mut slots);
            }
            Kind::Lectern => {
                containers[MAIN] = vec![None; 1];
                slots.extend(main(1, Role::Plain));
                // The open page (LecternBlockEntity.dataAccess).
                data = 1;
            }
            Kind::Mount(columns) => {
                containers[MAIN] = vec![None; 2 + columns * 3];
                for (index, slot) in ["saddle", "body"].into_iter().enumerate() {
                    slots.push(Slot {
                        container: MAIN,
                        index,
                        role: Role::MountEquip(slot),
                    });
                }
                slots.extend((2..2 + columns * 3).map(|index| Slot {
                    container: MAIN,
                    index,
                    role: Role::Plain,
                }));
                player(&mut slots);
            }
            Kind::ItemPicker => {
                containers[MAIN] = vec![None; 45];
                slots.extend(main(45, Role::CreativeDisplay));
                // addInventoryHotbarSlots.
                slots.extend((0..9).map(|index| Slot {
                    container: PLAYER,
                    index,
                    role: Role::Plain,
                }));
            }
            Kind::Merchant => {
                containers[MAIN] = vec![None; 2];
                slots.extend(main(2, Role::Plain));
                slots.push(Slot {
                    container: RESULT,
                    index: 0,
                    role: Role::MerchantResult,
                });
                player(&mut slots);
            }
            Kind::Brewing => {
                containers[MAIN] = vec![None; 5];
                for (index, role) in [
                    Role::Potion,
                    Role::Potion,
                    Role::Potion,
                    Role::Ingredient,
                    Role::BrewFuel,
                ]
                .into_iter()
                .enumerate()
                {
                    slots.push(Slot {
                        container: MAIN,
                        index,
                        role,
                    });
                }
                player(&mut slots);
                data = 4;
            }
            Kind::Furnace(_) => {
                containers[MAIN] = vec![None; 3];
                slots.extend([
                    Slot {
                        container: MAIN,
                        index: 0,
                        role: Role::Plain,
                    },
                    Slot {
                        container: MAIN,
                        index: 1,
                        role: Role::Fuel,
                    },
                    Slot {
                        container: MAIN,
                        index: 2,
                        role: Role::Output,
                    },
                ]);
                player(&mut slots);
                data = 4;
            }
        }
        containers[GRID] = vec![None; grid_size * grid_size];
        containers[RESULT] = vec![None; 1];
        Self {
            kind,
            registry_id: match kind {
                Kind::Mod(kind) => Some(kind.id.into()),
                _ => None,
            },
            slots,
            containers,
            grid_size,
            carried: None,
            data: vec![0; data],
            creative: false,
            selected: 0,
            drops: Vec::new(),
            dropped: Vec::new(),
            quick_status: 0,
            quick_type: 0,
            quick_slots: Vec::new(),
            loom_patterns: Vec::new(),
            sounds: Vec::new(),
            player_sounds: Vec::new(),
            crafted: Vec::new(),
            item_crafted: Vec::new(),
            crafted_counts: Vec::new(),
            merchant_remove_count: 0,
            brewed: Vec::new(),
            brewed_potion_hook: false,
            brewed_potion_takes: Vec::new(),
            item_smelted_hook: false,
            item_smelted: Vec::new(),
            item_smithed_hook: false,
            item_smithed: Vec::new(),
            output_removed: 0,
            equipped: Vec::new(),
            merchant: None,
            stone: Stonecutter::default(),
            ground: None,
            levels: 0,
            level_change: 0,
            anvil: Anvil::default(),
            combined: false,
            anvil_hook: false,
            anvil_craft: None,
            grind_hook: false,
            grind_take: None,
            grind_xp: -1,
            anvil_made: false,
            enchant_changed: false,
            enchant_button: None,
            may_build: true,
            mount: None,
        }
        .with_initial_data()
    }

    /// Data slots that do not start at 0: the enchanting table's clues are -1.
    fn with_initial_data(mut self) -> Self {
        if self.kind == Kind::Enchanting {
            self.data[4..].fill(-1);
        }
        self
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Registry identity of this menu, or the mount sentinel for a mount menu.
    /// Player inventory and the creative picker have no registered menu type.
    pub fn registry_id(&self) -> Option<&str> {
        self.registry_id.as_deref()
    }

    /// Set the identity obtained from the open window's menu registry.
    pub fn set_registry_id(&mut self, id: Option<&str>) {
        self.registry_id = id.map(Into::into);
    }

    /// `clickMenuButton`: the loom's pattern buttons and the stonecutter's
    /// recipes. False when refused.
    pub fn button(&mut self, env: &Env, id: i32) -> bool {
        if self.kind == Kind::Lectern {
            return self.lectern_button(env, id);
        }
        if self.kind == Kind::Enchanting {
            return self.enchant_click(id);
        }
        if self.kind == Kind::Stonecutter {
            // StonecutterMenu.clickMenuButton: an invalid index is taken too.
            if self.data[0] == id {
                return false;
            }
            if id >= 0 && (id as usize) < self.stone.recipes.len() {
                self.data[0] = id;
                self.stone_setup_result(env, id);
            }
            return true;
        }
        if self.kind != Kind::Loom || id < 0 || id as usize >= self.loom_patterns.len() {
            return false;
        }
        self.data[0] = id;
        let pattern = self.loom_patterns[id as usize].clone();
        self.loom_setup_result(env, &pattern);
        true
    }

    /// `LecternMenu.clickMenuButton`: the page buttons (1 back, 2 forward,
    /// 100 + n to page n) and Take Book (3), which puts the book in the
    /// inventory or drops it.
    fn lectern_button(&mut self, env: &Env, id: i32) -> bool {
        match id {
            100.. => self.set_data(0, id - 100),
            1 => self.set_data(0, self.data[0] - 1),
            2 => self.set_data(0, self.data[0] + 1),
            3 if self.may_build => {
                // removeItemNoUpdate: the whole book, and onBookItemRemove.
                let book = self.containers[MAIN][0].take();
                self.data[0] = 0;
                if let Some(mut book) = book
                    && !self.inventory_add(env, &mut book)
                {
                    self.drop_stack(&book);
                }
            }
            _ => return false,
        }
        true
    }

    /// `AbstractContainerMenu.setData`: the lectern's is
    /// `LecternBlockEntity.setPage`, kept within the book's pages.
    pub fn set_data(&mut self, id: usize, value: i32) {
        if self.kind == Kind::Lectern && id == 0 {
            // Mth.clamp(page, 0, pageCount - 1).
            let count = self.containers[MAIN][0].as_ref().map_or(0, book_pages) as i32;
            self.data[0] = value.max(0).min(count - 1);
            return;
        }
        self.data[id] = value;
    }

    /// Container slot `slot` of a mod menu type is left out by its screen.
    pub fn hide_slot(&mut self, slot: usize) {
        if matches!(self.kind, Kind::Mod(_)) && slot < self.slots.len() - 36 {
            self.slots[slot].role = Role::Hidden;
        }
    }

    /// `CrafterBlockEntity.setSlotState` (the slot state packet): only an
    /// empty slot can be disabled or enabled.
    pub fn set_slot_state(&mut self, slot: usize, enabled: bool) {
        if self.kind == Kind::CrafterBlock && slot < 9 && self.containers[MAIN][slot].is_none() {
            self.data[slot] = i32::from(!enabled);
        }
    }

    /// The menu after a fixture set its slots: the block crafter's slot
    /// listener has refreshed the result (`broadcastChanges`).
    pub fn settle(&mut self, env: &Env) {
        if self.kind == Kind::CrafterBlock {
            self.crafter_result(env);
        }
    }

    /// `CrafterMenu.refreshRecipeResult`.
    fn crafter_result(&mut self, env: &Env) {
        self.containers[RESULT][0] = env
            .recipes
            .find(env.loop_context, &self.containers[MAIN], 3)
            .map(|(_, stack)| stack)
            .filter(|s| s.count > 0);
    }

    /// `LoomMenu.setupResultSlot`.
    fn loom_setup_result(&mut self, env: &Env, pattern: &str) {
        let mut result = None;
        if let (Some(banner_stack), Some(dye)) = (
            self.containers[MAIN][0].as_ref(),
            self.containers[MAIN][1].as_ref(),
        ) && let Some(color) = env.banners.loom_dye(env.items, dye)
        {
            result = Some(banner::loom_result(banner_stack, &color, pattern));
        }
        self.containers[RESULT][0] = result;
    }

    /// `LoomMenu.slotsChanged`.
    fn loom_changed(&mut self, env: &Env) {
        let has_inputs = self.containers[MAIN][0].is_some() && self.containers[MAIN][1].is_some();
        if !has_inputs {
            self.containers[RESULT][0] = None;
            self.loom_patterns.clear();
            self.data[0] = -1;
            return;
        }
        let selected = self.data[0];
        let valid = selected >= 0 && (selected as usize) < self.loom_patterns.len();
        let pattern_item = self.containers[MAIN][2].clone();
        let previous = std::mem::replace(
            &mut self.loom_patterns,
            env.banners.selectable(pattern_item.as_ref()),
        );
        let shown = if self.loom_patterns.len() == 1 {
            self.data[0] = 0;
            Some(self.loom_patterns[0].clone())
        } else if !valid {
            self.data[0] = -1;
            None
        } else {
            let value = previous[selected as usize].clone();
            match self.loom_patterns.iter().position(|p| *p == value) {
                Some(i) => {
                    self.data[0] = i as i32;
                    Some(value)
                }
                None => {
                    self.data[0] = -1;
                    None
                }
            }
        };
        match shown {
            Some(pattern) => {
                let full = self.containers[MAIN][0]
                    .as_ref()
                    .is_some_and(banner::has_max_layers);
                if full {
                    self.data[0] = -1;
                    self.containers[RESULT][0] = None;
                } else {
                    self.loom_setup_result(env, &pattern);
                }
            }
            None => self.containers[RESULT][0] = None,
        }
    }

    /// `StonecutterMenu.slotsChanged`: a new input item lists its recipes
    /// (`setupRecipeList`), with none selected and no result.
    fn stone_changed(&mut self, env: &Env) {
        let input = self.containers[MAIN][0].as_ref().map(|s| s.item.clone());
        if input == self.stone.input {
            return;
        }
        self.stone.input = input;
        self.data[0] = -1;
        self.containers[RESULT][0] = None;
        self.stone.recipes = match &self.containers[MAIN][0] {
            Some(stack) => env
                .station_recipes
                .stonecutting
                .iter()
                .enumerate()
                .filter(|(_, r)| r.input.test_in(env.loop_context, stack))
                .map(|(i, _)| i)
                .collect(),
            None => Vec::new(),
        };
    }

    /// `StonecutterMenu.setupResultSlot`.
    fn stone_setup_result(&mut self, env: &Env, index: i32) {
        let recipe = usize::try_from(index)
            .ok()
            .and_then(|i| self.stone.recipes.get(i).copied());
        self.stone.used = recipe;
        self.containers[RESULT][0] =
            recipe.map(|r| env.station_recipes.stonecutting[r].result.clone());
    }

    /// `Slot.setChanged` on slot `s`: the loom recomputes from its inputs,
    /// the stonecutter lists its recipes, the block crafter refreshes its
    /// result.
    fn slot_changed(&mut self, env: &Env, s: usize) {
        // MerchantContainer.setChanged: every slot of it, the result too.
        if self.kind == Kind::Merchant && self.slots[s].container != PLAYER {
            self.merchant_update(env);
            return;
        }
        // SmithingMenu.slotsChanged runs for its result container too.
        if self.kind == Kind::Smithing && self.slots[s].container == RESULT {
            self.smith_error();
            return;
        }
        // CartographyTableMenu: its result container calls slotsChanged too.
        if self.kind == Kind::Cartography && self.slots[s].container != PLAYER {
            self.carto_changed(env);
            return;
        }
        if self.slots[s].container != MAIN {
            return;
        }
        match self.kind {
            Kind::Loom => self.loom_changed(env),
            Kind::Stonecutter => self.stone_changed(env),
            // GrindstoneMenu.createResult.
            Kind::Grindstone => {
                self.containers[RESULT][0] = crate::stations::grindstone_result(
                    env.enchantments,
                    env.items,
                    self.containers[MAIN][0].as_ref(),
                    self.containers[MAIN][1].as_ref(),
                );
            }
            Kind::CrafterBlock => self.crafter_result(env),
            Kind::Anvil => self.anvil_result(env),
            Kind::Enchanting => self.enchant_changed(env),
            Kind::Smithing => {
                self.smith_result(env);
                self.smith_error();
            }
            _ => {}
        }
    }

    /// `slotsChanged(player.getInventory())`, which a bundle click calls
    /// (`BundleItem.broadcastChangesOnContainerMenu`): the menus that don't
    /// check which container changed recompute from their inputs.
    fn inventory_slots_changed(&mut self, env: &Env) {
        match self.kind {
            Kind::Inventory | Kind::Crafting => self.recompute(env),
            Kind::Loom => self.loom_changed(env),
            Kind::Stonecutter => self.stone_changed(env),
            Kind::Cartography => self.carto_changed(env),
            Kind::Merchant => self.merchant_update(env),
            // ItemCombinerMenu.slotsChanged only for its inputs; the
            // smithing table's hasRecipeError always.
            Kind::Smithing => self.smith_error(),
            _ => {}
        }
    }

    /// `EnchantmentMenu.clickMenuButton`'s checks: the lapis, the cost and
    /// the levels (creative pays nothing). The enchanting itself needs a
    /// level and is the caller's (`enchant_button`).
    fn enchant_click(&mut self, id: i32) -> bool {
        let Some(i) = usize::try_from(id).ok().filter(|&i| i < 3) else {
            return false;
        };
        let need = id + 1;
        let lapis = self.containers[MAIN][1]
            .as_ref()
            .map_or(0, |l| l.count as i32);
        if lapis < need && !self.creative {
            return false;
        }
        if self.data[i] <= 0
            || self.containers[MAIN][0].is_none()
            || (self.levels < need || self.levels < self.data[i]) && !self.creative
        {
            return false;
        }
        self.enchant_button = Some(i);
        true
    }

    /// `EnchantmentMenu.slotsChanged`: no costs or clues without an
    /// enchantable item; with one, a level fills them (`enchant_changed`).
    fn enchant_changed(&mut self, env: &Env) {
        let enchantable = self.containers[MAIN][0].as_ref().is_some_and(|s| {
            env.enchantments.enchantable(&s.item) > 0 && s.enchantments().is_empty()
        });
        if enchantable {
            self.enchant_changed = true;
        } else {
            self.data[..3].fill(0);
            self.data[4..].fill(-1);
        }
    }

    /// `AnvilMenu.createResult`.
    fn anvil_result(&mut self, env: &Env) {
        let hover = self.containers[MAIN][0]
            .as_ref()
            .map(|i| crate::stations::hover_name(&|id| env.names.display_item(id), i))
            .unwrap_or_default();
        let out = crate::stations::anvil_result(
            env.enchantments,
            env.items,
            self.containers[MAIN][0].as_ref(),
            self.containers[MAIN][1].as_ref(),
            self.anvil.name.as_deref(),
            &hover,
            self.creative,
        );
        // onlyRenaming is reset, repairItemCountCost only where an input is.
        self.anvil.only_renaming = out.only_renaming;
        if self.containers[MAIN][0].is_some() {
            self.anvil.repair_count = out.repair_item_count_cost;
        }
        self.data[0] = out.cost;
        self.containers[RESULT][0] = out.result;
        self.anvil_made = true;
    }

    /// The anvil's result, cost and state as `createResult` left them.
    fn anvil_state(&self) -> crate::stations::AnvilResult {
        crate::stations::AnvilResult {
            result: self.containers[RESULT][0].clone(),
            cost: self.data[0],
            repair_item_count_cost: self.anvil.repair_count,
            only_renaming: self.anvil.only_renaming,
        }
    }

    /// Whether `createResult` ran since the last call (an input or the name
    /// changed, even back to what it was, or to no visible change).
    pub fn anvil_created(&mut self) -> bool {
        std::mem::take(&mut self.anvil_made)
    }

    /// `AnvilUpdateEvent`'s question about the result `createResult` made,
    /// or `None` with no left input, where vanilla has no result.
    pub fn anvil_update(&self) -> Option<AnvilUpdate> {
        let left = self.containers[MAIN][0].clone()?;
        let right = self.containers[MAIN][1].clone();
        Some(AnvilUpdate::new(
            left,
            right,
            self.anvil.name.clone(),
            &self.anvil_state(),
        ))
    }

    /// The mods' answer to [`Self::anvil_update`] (see [`AnvilUpdate::answer`]).
    pub fn anvil_updated(&mut self, update: AnvilUpdate, cancel: bool) {
        let mut state = self.anvil_state();
        update.answer(cancel, &mut state);
        self.anvil.only_renaming = state.only_renaming;
        self.anvil.repair_count = state.repair_item_count_cost;
        self.data[0] = state.cost;
        self.containers[RESULT][0] = state.result;
    }

    /// `GrindstoneEvent.OnPlaceItem`'s question about the result
    /// `createResult` just made.
    pub fn grindstone_place(&self) -> GrindstonePlace {
        GrindstonePlace {
            top: self.containers[MAIN][0].clone(),
            bottom: self.containers[MAIN][1].clone(),
            vanilla_output: self.containers[RESULT][0].clone(),
            output: None,
            xp: -1,
        }
    }

    /// The mods' answer to [`Self::grindstone_place`]: a cancel leaves no
    /// result; an output replaces vanilla's, and its `xp` (when 0 or more)
    /// is what taking it gives; otherwise vanilla's result and experience stay.
    pub fn grindstone_placed(&mut self, place: GrindstonePlace, cancel: bool) {
        self.grind_xp = -1;
        if cancel {
            self.containers[RESULT][0] = None;
        } else if let Some(output) = place.output {
            self.grind_xp = place.xp.max(-1);
            self.containers[RESULT][0] = Some(output);
        }
    }

    /// The rest of a taken grindstone result's `onTake` once the mods
    /// answered `grind_take`: cancelled, the inputs stay and nothing is
    /// given. Otherwise the inputs become `new_top` and `new_bottom`, and
    /// the experience is the mods' `xp` when 0 or more (returned, given as
    /// is) or vanilla's (`ground`, rolled by the caller).
    pub fn grindstone_taken(
        &mut self,
        env: &Env,
        take: GrindstoneTake,
        cancel: bool,
    ) -> Option<i32> {
        if cancel {
            return None;
        }
        if take.xp < 0 {
            self.ground = Some(crate::stations::grindstone_xp_base(
                env.enchantments,
                take.top.as_ref(),
                take.bottom.as_ref(),
            ));
        }
        self.put(env, 0, take.new_top);
        self.put(env, 1, take.new_bottom);
        (take.xp >= 0).then_some(take.xp)
    }

    /// `AnvilMenu.setItemName` (`ServerboundRenameItemPacket`): false when
    /// the name is refused or unchanged.
    /// `BeaconMenu.updateEffects` (`ServerboundSetBeaconPacket`): with a
    /// payment, powers `BeaconBlockEntity.validateEffects` allows for the
    /// pyramid's levels (data 0) become data 1 and 2 and the payment is used.
    /// Each power is its `mob_effect` id and name. False when refused.
    pub fn beacon_update(
        &mut self,
        env: &Env,
        primary: Option<(i32, &str)>,
        secondary: Option<(i32, &str)>,
    ) -> bool {
        if self.kind != Kind::Beacon || self.containers[MAIN][0].is_none() {
            return false;
        }
        let levels = self.data[0];
        let (p, s) = (
            beacon_required_levels(primary.map(|e| e.1)),
            beacon_required_levels(secondary.map(|e| e.1)),
        );
        let valid = !(secondary.is_some() && levels < 4)
            && p <= levels
            && s <= levels
            && p < 4
            && !(s != 0 && s < 4 && primary.map(|e| e.0) != secondary.map(|e| e.0));
        if !valid {
            return false;
        }
        self.data[1] = primary.map_or(0, |e| e.0 + 1);
        self.data[2] = secondary.map_or(0, |e| e.0 + 1);
        self.remove(env, 0, 1);
        true
    }

    pub fn rename(&mut self, env: &Env, name: &str) -> bool {
        // StringUtil.filterText, then validateName's length limit.
        let name: String = name
            .chars()
            .filter(|&c| c != '\u{a7}' && c >= ' ' && c != '\u{7f}')
            .collect();
        if self.kind != Kind::Anvil
            || name.encode_utf16().count() > 50
            || self.anvil.name.as_deref() == Some(&name)
        {
            return false;
        }
        if let Some(result) = self.containers[RESULT][0].as_mut() {
            result.data_mut().custom_name = (!name.trim().is_empty())
                .then(|| horizon_core::nbt::Tag::String(name.as_str().into()));
            result.tidy();
        }
        self.anvil.name = Some(name);
        self.anvil_result(env);
        true
    }

    /// `SmithingMenu.createResult`.
    fn smith_result(&mut self, env: &Env) {
        let [a, b, c] = [0, 1, 2].map(|i| self.containers[MAIN][i].as_ref());
        self.containers[RESULT][0] =
            env.station_recipes
                .smithing_result(env.loop_context, env.enchantments, a, b, c);
    }

    /// `SmithingMenu.slotsChanged`'s `hasRecipeError`: every input filled and no result.
    fn smith_error(&mut self) {
        let error = self.containers[MAIN].iter().all(Option::is_some)
            && self.containers[RESULT][0].is_none();
        self.data[0] = i32::from(error);
    }

    /// `CartographyTableMenu.slotsChanged` and `setupResultSlot`: a result
    /// only where the level has the map (`access.execute`), with the post
    /// processing its take runs (`MAP_POST_PROCESSING`: 0 lock, 1 scale).
    fn carto_changed(&mut self, env: &Env) {
        let [map, extra] = [0, 1].map(|i| self.containers[MAIN][i].clone());
        if self.containers[RESULT][0].is_some() && (map.is_none() || extra.is_none()) {
            self.containers[RESULT][0] = None;
            return;
        }
        let (Some(map), Some(extra), Some(maps)) = (map, extra, env.maps) else {
            return;
        };
        let Some((locked, scale)) = map
            .data()
            .and_then(|d| d.map_id)
            .and_then(|id| maps.state(id))
        else {
            return;
        };
        let extendable = env
            .items
            .item_tags
            .get("minecraft:extendable_maps")
            .is_some_and(|t| t.contains(&*map.item));
        let copy = |count: u32, post: Option<u8>| {
            let mut out = ItemStack {
                count,
                ..map.clone()
            };
            if post.is_some() {
                out.data_mut().map_post_processing = post;
            }
            out
        };
        self.containers[RESULT][0] = match &*extra.item {
            "minecraft:paper" if extendable && !locked && scale < 4 => Some(copy(1, Some(1))),
            "minecraft:glass_pane" if !locked => Some(copy(1, Some(0))),
            "minecraft:map" => Some(copy(2, None)),
            _ => None,
        };
    }

    /// `MapItem.onCraftedPostProcess`, shared by menu takes and system crafting:
    /// remove the directive and name the level's scaled or locked saved copy.
    pub fn map_crafted(maps: Option<&dyn Maps>, stack: &mut ItemStack) {
        if &*stack.item != "minecraft:filled_map" {
            return;
        }
        let Some(post) = stack.data().and_then(|d| d.map_post_processing) else {
            return;
        };
        stack.data_mut().map_post_processing = None;
        let id = stack.data().and_then(|d| d.map_id);
        if let (Some(maps), Some(id)) = (maps, id)
            && let Some(new) = maps.derive(id, post == 1)
        {
            stack.data_mut().map_id = Some(new);
        }
        stack.tidy();
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn slot(&self, i: usize) -> Option<&ItemStack> {
        let s = self.slots[i];
        self.containers[s.container][s.index].as_ref()
    }

    /// Load an existing container slot without applying insertion limits.
    pub fn load_slot(&mut self, i: usize, stack: Option<ItemStack>) {
        let slot = self.slots[i];
        self.containers[slot.container][slot.index] = stack;
    }

    /// Sets a slot as a test fixture does (`Slot.set`).
    pub fn set_slot(&mut self, env: &Env, i: usize, stack: Option<ItemStack>) {
        if self.kind == Kind::Lectern {
            // LecternBlockEntity.setBook: the book its slot can't set.
            self.containers[MAIN][0] = stack;
            return;
        }
        self.put(env, i, stack);
    }

    pub fn player_item(&self, i: usize) -> Option<&ItemStack> {
        self.containers[PLAYER][i].as_ref()
    }

    pub fn set_player_item(&mut self, i: usize, stack: Option<ItemStack>) {
        self.containers[PLAYER][i] = stack;
    }

    /// `Inventory.add`: the leftover stays in `stack`.
    pub fn give(&mut self, env: &Env, stack: &mut ItemStack) -> bool {
        self.inventory_add(env, stack)
    }

    /// `Inventory.contains(stack)`.
    pub fn player_contains(&self, stack: &ItemStack) -> bool {
        self.containers[PLAYER]
            .iter()
            .flatten()
            .any(|have| have.same_item_same_components(stack))
    }

    // ---- item helpers ----

    fn max_stack(env: &Env, stack: &ItemStack) -> u32 {
        item_max_stack(env.items, stack)
    }

    fn damageable(env: &Env, stack: &ItemStack) -> bool {
        env.items.get(&stack.item).is_some_and(|i| i.max_damage > 0)
    }

    /// `ItemStack.isStackable`.
    fn stackable(env: &Env, stack: &ItemStack) -> bool {
        stackable(env.items, stack)
    }

    fn is_bucket(stack: &ItemStack) -> bool {
        &*stack.item == "minecraft:bucket"
    }

    fn is_fuel(env: &Env, stack: &ItemStack) -> bool {
        env.items.get(&stack.item).is_some_and(|i| i.fuel.is_some())
    }

    // ---- Slot ----

    fn may_place(&self, env: &Env, s: usize, stack: &ItemStack) -> bool {
        if self.slots[s].role == Role::Hidden {
            return false;
        }
        let own = self.vanilla_may_place(env, s, stack);
        self.slot_rules(env, s, stack, own, self.vanilla_slot_max(s, stack))
            .0
    }

    fn vanilla_may_place(&self, env: &Env, s: usize, stack: &ItemStack) -> bool {
        match self.slots[s].role {
            Role::Plain | Role::CreativeDisplay => true,
            Role::Armor(eq) => env
                .items
                .get(&stack.item)
                .and_then(|i| {
                    let slot = match i.equip_slot.as_deref()? {
                        "feet" => 0,
                        "legs" => 1,
                        "chest" => 2,
                        "head" => 3,
                        _ => return None,
                    };
                    let allowed = i
                        .equip_allowed
                        .as_ref()
                        .is_none_or(|a| a.contains("minecraft:player"));
                    Some(slot == eq && allowed)
                })
                .unwrap_or(false),
            // LivingEntity.isEquippableInSlot: the item's equippable slot,
            // and the mount among its allowed entities.
            Role::MountEquip(slot) => env.items.get(&stack.item).is_some_and(|i| {
                i.equip_slot.as_deref() == Some(slot)
                    && self
                        .mount
                        .as_deref()
                        .is_some_and(|m| i.equip_allowed.as_ref().is_none_or(|a| a.contains(m)))
            }),
            Role::Shulker => fits_inside_container_items(stack),
            Role::Mod | Role::ModResult => self.slots[s].role == Role::Mod,
            Role::Hidden => false,
            Role::Fuel => Self::is_fuel(env, stack) || Self::is_bucket(stack),
            Role::Output | Role::CraftResult | Role::Inert => false,
            Role::CrafterIn => self.data.get(self.slots[s].index) != Some(&1),
            Role::Potion => {
                env.brewing.is_bottle(stack)
                    || env
                        .items
                        .item_tags
                        .get("minecraft:brewing_potion_inputs")
                        .is_some_and(|m| m.contains(&*stack.item))
            }
            Role::Ingredient => env.brewing.is_reagent(stack),
            Role::BrewFuel => env
                .items
                .get(&stack.item)
                .is_some_and(|i| i.brewing_fuel.is_some()),
            Role::LoomBanner => Self::is_banner(stack),
            Role::LoomDye => env.banners.loom_dye(env.items, stack).is_some(),
            Role::LoomPattern => env.banners.is_pattern_item(env.items, stack),
            Role::GrindInput => crate::stations::grindstone_accepts(env.items, stack),
            Role::SmithInput(i) => env
                .station_recipes
                .smithing_accepts(env.loop_context, i, stack),
            Role::EnchantItem => true,
            Role::Lapis => &*stack.item == "minecraft:lapis_lazuli",
            Role::BeaconPay => env
                .items
                .item_tags
                .get("minecraft:beacon_payment_items")
                .is_some_and(|t| t.contains(&*stack.item)),
            Role::CartoMap => stack.data().is_some_and(|d| d.map_id.is_some()),
            Role::CartoExtra => Self::is_carto_extra(stack),
            Role::LoomResult
            | Role::MerchantResult
            | Role::StoneResult
            | Role::GrindResult
            | Role::CombineResult
            | Role::CartoResult => false,
        }
    }

    /// What the cartography table's second slot takes.
    fn is_carto_extra(stack: &ItemStack) -> bool {
        matches!(
            &*stack.item,
            "minecraft:paper" | "minecraft:map" | "minecraft:glass_pane"
        )
    }

    /// `BannerItem`.
    fn is_banner(stack: &ItemStack) -> bool {
        stack.item.ends_with("_banner")
    }

    fn may_pickup(&self, env: &Env, s: usize) -> bool {
        match self.slots[s].role {
            Role::Inert | Role::Hidden => false,
            // AnvilMenu.mayPickup: the levels to pay, and a price.
            Role::CombineResult if self.kind == Kind::Anvil => {
                (self.creative || self.levels >= self.data[0]) && self.data[0] > 0
            }
            Role::MerchantResult => self
                .merchant
                .as_ref()
                .and_then(|m| m.offers.get(m.active?))
                .is_some_and(|o| {
                    let (a, b) = (
                        self.containers[MAIN][0].as_ref(),
                        self.containers[MAIN][1].as_ref(),
                    );
                    o.satisfied_by(env.items, a, b) || o.satisfied_by(env.items, b, a)
                }),
            Role::Armor(_) | Role::MountEquip(_) => match self.slot(s) {
                Some(stack) => self.creative || !env.enchantments.prevents_armor_change(stack),
                None => true,
            },
            _ => true,
        }
    }

    /// A result slot (`FurnaceResultSlot`, `ResultSlot`): nothing goes in, and
    /// what's taken from it can't be put back.
    pub fn is_result(&self, s: usize) -> bool {
        matches!(self.slots[s].role, Role::Output | Role::CraftResult)
    }

    /// How many of `stack` slot `s` holds: `Slot.getMaxStackSize(stack)` where `Slot.mayPlace`
    /// takes it, else none.
    pub fn slot_holds(&self, env: &Env, s: usize, stack: &ItemStack) -> u32 {
        match self.may_place(env, s, stack) {
            true => self.slot_max(env, s, stack),
            false => 0,
        }
    }

    /// `Slot.mayPickup`: whether slot `s` gives up what it holds.
    pub fn slot_gives(&self, env: &Env, s: usize) -> bool {
        self.may_pickup(env, s)
    }

    /// Slot `s` takes one more item of `stack` (`Slot.mayPlace`, and room).
    pub fn takes_one(&self, env: &Env, s: usize, stack: &ItemStack) -> bool {
        self.may_place(env, s, stack)
            && self.slot(s).is_none_or(|have| {
                have.same_item_same_components(stack) && have.count < self.slot_max(env, s, have)
            })
    }

    /// The slots `quickMoveStack` would put slot `index`'s items in: one item
    /// of a plain slot, the whole of a result slot, partial stacks first. Empty
    /// when they don't all fit. This runs the move, so call it on a menu that
    /// is thrown away afterwards.
    pub fn quick_move_landing(&mut self, env: &Env, index: usize) -> Vec<usize> {
        let Some(mut stack) = self.slot(index).cloned() else {
            return Vec::new();
        };
        if !self.is_result(index) {
            stack.count = 1;
            let slot = self.slots[index];
            self.containers[slot.container][slot.index] = Some(stack.clone());
        }
        let before: Vec<Option<ItemStack>> = (0..self.slots.len())
            .map(|i| self.slot(i).cloned())
            .collect();
        self.quick_move(env, index);
        let (mut partial, mut empty) = (Vec::new(), Vec::new());
        let mut moved = 0;
        for i in (0..self.slots.len()).filter(|&i| i != index && !self.is_result(i)) {
            let had = before[i].as_ref().map_or(0, |b| b.count);
            let got = self
                .slot(i)
                .filter(|now| now.same_item_same_components(&stack))
                .map_or(0, |now| now.count.saturating_sub(had));
            if got > 0 {
                moved += got;
                if had > 0 {
                    partial.push(i);
                } else {
                    empty.push(i);
                }
            }
        }
        if moved < stack.count {
            return Vec::new();
        }
        partial.extend(empty);
        partial
    }

    fn slot_rules(
        &self,
        env: &Env,
        s: usize,
        stack: &ItemStack,
        own: bool,
        max: u32,
    ) -> (bool, u32) {
        let Some(id) = self.registry_id() else {
            return (own, max.min(Self::max_stack(env, stack)));
        };
        apply_menu_slot_rules(
            env.loop_context,
            id,
            self.slots[s].index,
            stack,
            own,
            max,
            Self::max_stack(env, stack),
        )
    }

    /// `Slot.getMaxStackSize(stack)`.
    fn slot_max(&self, env: &Env, s: usize, stack: &ItemStack) -> u32 {
        self.slot_rules(
            env,
            s,
            stack,
            self.vanilla_may_place(env, s, stack),
            self.vanilla_slot_max(s, stack),
        )
        .1
    }

    fn vanilla_slot_max(&self, s: usize, stack: &ItemStack) -> u32 {
        match self.slots[s].role {
            Role::Armor(_)
            | Role::MountEquip(_)
            | Role::Potion
            | Role::EnchantItem
            | Role::BeaconPay => 1,
            // The lectern's bookAccess holds one.
            Role::Plain if self.kind == Kind::Lectern && self.slots[s].container == MAIN => 1,
            Role::Fuel if Self::is_bucket(stack) => 1,
            Role::Mod | Role::ModResult => CONTAINER_MAX,
            // `Container.getMaxStackSize` is vanilla's 99, which no data pack's
            // item reaches; a native mod's item may, and its own size is the cap.
            _ => u32::MAX,
        }
    }

    /// `Container.setItem` (a grid change recomputes the result).
    fn put(&mut self, env: &Env, s: usize, mut stack: Option<ItemStack>) {
        tidy(&mut stack);
        let slot = self.slots[s];
        // LecternBlockEntity's bookAccess.setItem does nothing.
        if self.kind == Kind::Lectern && slot.container == MAIN {
            return;
        }
        // SimpleContainer / BaseContainerBlockEntity.setItem: limitSize to
        // getMaxStackSize(stack). The loom's result is a SimpleContainer too.
        if (slot.container == MAIN || slot.role == Role::LoomResult)
            && let Some(st) = stack.as_mut()
        {
            st.count = st.count.min(Self::max_stack(env, st));
            // The beacon's container holds one item (getMaxStackSize 1).
            if self.kind == Kind::Beacon {
                st.count = st.count.min(1);
            }
        }
        // CrafterBlockEntity.setItem: filling a disabled (empty) slot enables it.
        if self.kind == Kind::CrafterBlock
            && slot.container == MAIN
            && self.data[slot.index] == 1
            && self.containers[MAIN][slot.index].is_none()
        {
            self.data[slot.index] = 0;
        }
        self.containers[slot.container][slot.index] = stack;
        if slot.container == GRID {
            self.recompute(env);
        }
        self.slot_changed(env, s);
    }

    /// `Slot.setByPlayer(stack)`: the slot's item is the previous one.
    fn set_by_player(&mut self, env: &Env, s: usize, stack: Option<ItemStack>) {
        let previous = self.slot(s).cloned();
        self.set_by_player_from(env, s, stack, previous.as_ref());
    }

    /// `Slot.setByPlayer(stack, previous)`: `ArmorSlot` and the inventory
    /// menu's off-hand slot first call the owner's `onEquipItem`, which acts
    /// only when the item changes (`isSameItemSameComponents`).
    fn set_by_player_from(
        &mut self,
        env: &Env,
        s: usize,
        mut stack: Option<ItemStack>,
        previous: Option<&ItemStack>,
    ) {
        tidy(&mut stack);
        if let Some(slot) = self.equipment_slot(s) {
            let same = match (previous.filter(|p| p.count > 0), stack.as_ref()) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_item_same_components(b),
                _ => false,
            };
            if !same {
                self.equipped.push((slot, stack.clone()));
            }
        }
        self.put(env, s, stack);
    }

    /// The `EquipmentSlot` of an `ArmorSlot` or of `InventoryMenu`'s off-hand
    /// slot (the only menu slot over inventory slot 40).
    fn equipment_slot(&self, s: usize) -> Option<&'static str> {
        let slot = self.slots[s];
        match slot.role {
            Role::Armor(eq) => Some(["feet", "legs", "chest", "head"][eq]),
            _ if slot.container == PLAYER && slot.index == OFFHAND => Some("offhand"),
            _ => None,
        }
    }

    /// `Slot.remove`: the container's `removeItem`.
    fn remove(&mut self, env: &Env, s: usize, amount: u32) -> Option<ItemStack> {
        let slot = self.slots[s];
        if slot.role == Role::MerchantResult {
            let count = amount.min(self.slot(s).map_or(0, |stack| stack.count));
            self.merchant_remove_count = self.merchant_remove_count.wrapping_add(count as i32);
        }
        // ResultContainer.removeItem takes the whole stack; the loom's result
        // is a SimpleContainer slot and splits.
        if slot.container == RESULT && slot.role != Role::LoomResult {
            return self.containers[RESULT][0].take();
        }
        let cell = &mut self.containers[slot.container][slot.index];
        let out = cell
            .as_mut()
            .filter(|c| c.count > 0)
            .map(|c| split(c, amount));
        tidy(cell);
        // FurnaceResultSlot.remove counts what it gave.
        if slot.role == Role::Output {
            self.output_removed += out.as_ref().map_or(0, |o| o.count);
        }
        // LecternBlockEntity: an emptied book slot is onBookItemRemove (page 0).
        if self.kind == Kind::Lectern && slot.container == MAIN && cell.is_none() {
            self.data[0] = 0;
        }
        if out.is_some() && slot.container == GRID {
            self.recompute(env);
        }
        // SimpleContainer.removeItem calls setChanged when it took something;
        // MerchantContainer.removeItem updates the result after a payment slot.
        if out.is_some()
            && matches!(
                self.kind,
                Kind::Loom
                    | Kind::Merchant
                    | Kind::Stonecutter
                    | Kind::Grindstone
                    | Kind::Anvil
                    | Kind::Smithing
                    | Kind::Enchanting
                    | Kind::Cartography
            )
        {
            self.slot_changed(env, s);
        }
        out
    }

    /// The anvil menu as it is before a click that may take its result.
    pub fn anvil_undo(&self) -> AnvilUndo {
        AnvilUndo {
            inputs: self.containers[MAIN].clone(),
            result: self.containers[RESULT][0].clone(),
            cost: self.data[0],
            player: self.containers[PLAYER].clone(),
            carried: self.carried.clone(),
        }
    }

    /// Refuses a take that `anvil_hook` deferred (a cancelled
    /// `AnvilCraftEvent.Pre`): the result goes back in its slot and the
    /// inputs, cost, inventory and cursor are as `undo` held them, so nothing
    /// the click did (cursor, shift-click, swap, drop) is left.
    pub fn anvil_cancel(&mut self, undo: AnvilUndo) {
        self.containers[MAIN] = undo.inputs;
        self.containers[RESULT][0] = undo.result;
        self.data[0] = undo.cost;
        self.containers[PLAYER] = undo.player;
        self.carried = undo.carried;
        self.anvil_craft = None;
        self.dropped.clear();
    }

    /// `AnvilMenu.onTake`: the levels, the material used up, then the input.
    pub fn anvil_take(&mut self, env: &Env) {
        let cost = self.data[0];
        if !self.creative {
            // Player.giveExperienceLevels(-cost).
            self.level_change -= cost;
            self.drops.push(format!(
                "levels{}{}",
                if cost <= 0 { "+" } else { "" },
                -cost
            ));
        }
        let repair = self.anvil.repair_count;
        if repair > 0 {
            let addition = self.containers[MAIN][1]
                .clone()
                .filter(|a| a.count > repair)
                .map(|mut a| {
                    a.count -= repair;
                    a
                });
            self.put(env, 1, addition);
        } else if !self.anvil.only_renaming {
            self.put(env, 1, None);
        }
        self.data[0] = 0;
        self.put(env, 0, None);
        self.combined = true;
    }

    /// `CraftingMenu.slotChangedCraftingGrid`.
    fn recompute(&mut self, env: &Env) {
        if self.grid_size == 0 {
            return;
        }
        let result = env
            .recipes
            .find(env.loop_context, &self.containers[GRID], self.grid_size)
            .and_then(|(id, stack)| {
                if env.limited_crafting {
                    // `RecipeCraftingHolder.setRecipeUsed`: special recipes or known recipes craft.
                    let is_special = env.recipes.recipe(id).is_some_and(|r| r.is_special());
                    let known = env.known_recipes.is_some_and(|book| book.knows(id));
                    if !is_special && !known {
                        return None;
                    }
                }
                Some(stack)
            });
        self.containers[RESULT][0] = result;
    }

    /// `FurnaceResultSlot.checkTakeAchievements`: what `carried` and
    /// `removeCount` were, kept for `ItemSmeltedEvent` when a server caller
    /// listens; the count starts again.
    fn furnace_take(&mut self, env: &Env, carried: &ItemStack) {
        let removed = std::mem::take(&mut self.output_removed);
        if self.item_smelted_hook && env.loop_context.side() == LoopSide::Server {
            let carried = (carried.count > 0).then(|| carried.clone());
            self.item_smelted.push((carried, removed));
        }
    }

    /// A station's take sound, gated by its menu's `lastSoundTime`: once per
    /// tick, so a shift-click that takes many results plays it once.
    fn take_sound(&mut self, sound: &'static str) {
        if !self.sounds.contains(&sound) {
            self.sounds.push(sound);
        }
    }

    /// `ResultSlot.onTake` and the other `Slot.onTake` overrides.
    fn on_take(&mut self, env: &Env, s: usize, taken: &ItemStack) {
        if self.slots[s].role == Role::Output {
            self.furnace_take(env, taken);
            return;
        }
        if self.slots[s].role == Role::Potion {
            if self.brewed_potion_hook && env.loop_context.side() == LoopSide::Server {
                self.brewed_potion_takes.push(taken.clone());
            }
            // BrewingStandMenu.PotionSlot.onTake: a stack with potion contents.
            if matches!(
                &*taken.item,
                "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion"
            ) {
                self.brewed.push(taken.clone());
            }
            return;
        }
        if self.slots[s].role == Role::MerchantResult {
            self.merchant_check_take_achievements(taken);
            self.merchant_take(env);
            return;
        }
        if self.slots[s].role == Role::StoneResult {
            // StonecutterMenu's result onTake: awardUsedRecipes, one input
            // used up, the result again for the selected recipe.
            if let Some(r) = self.stone.used {
                self.crafted.push((
                    env.station_recipes.stonecutting[r].id.clone(),
                    vec![self.containers[MAIN][0].clone()],
                ));
            }
            if self.remove(env, 0, 1).is_some() {
                self.stone_setup_result(env, self.data[0]);
            }
            self.take_sound("minecraft:ui.stonecutter.take_result");
            return;
        }
        if self.slots[s].role == Role::GrindResult {
            if self.grind_hook {
                self.grind_take = Some(GrindstoneTake {
                    top: self.containers[MAIN][0].clone(),
                    bottom: self.containers[MAIN][1].clone(),
                    xp: self.grind_xp,
                    new_top: None,
                    new_bottom: None,
                });
                return;
            }
            // GrindstoneMenu's result onTake: the experience (from the inputs
            // before they go), then both inputs cleared.
            self.ground = Some(crate::stations::grindstone_xp_base(
                env.enchantments,
                self.containers[MAIN][0].as_ref(),
                self.containers[MAIN][1].as_ref(),
            ));
            self.put(env, 0, None);
            self.put(env, 1, None);
            return;
        }
        if self.slots[s].role == Role::CombineResult && self.kind == Kind::Anvil {
            if self.anvil_hook {
                self.anvil_craft = Some(AnvilCraft {
                    left: self.containers[MAIN][0].clone(),
                    right: self.containers[MAIN][1].clone(),
                    output: taken.clone(),
                });
                return;
            }
            self.anvil_take(env);
            return;
        }
        if self.slots[s].role == Role::CombineResult {
            // SmithingMenu.onTake: NeoForge's ItemSmithedEvent first, then
            // awardUsedRecipes, then one of each input.
            if self.kind == Kind::Smithing {
                let inputs = [0, 1, 2].map(|i| self.containers[MAIN][i].clone());
                if self.item_smithed_hook
                    && env.loop_context.side() == LoopSide::Server
                    && taken.count > 0
                {
                    self.item_smithed.push((inputs.clone(), taken.clone()));
                }
                if let Some((id, _)) = env.station_recipes.smithing_recipe(
                    env.loop_context,
                    env.enchantments,
                    inputs[0].as_ref(),
                    inputs[1].as_ref(),
                    inputs[2].as_ref(),
                ) {
                    self.crafted.push((id.to_owned(), inputs.to_vec()));
                }
            }
            for i in 0..3 {
                if let Some(mut stack) = self.containers[MAIN][i].clone() {
                    stack.count -= 1;
                    self.put(env, i, Some(stack));
                }
            }
            self.combined = true;
            return;
        }
        if self.slots[s].role == Role::CartoResult {
            // CartographyTableMenu's result onTake: one of each input, the
            // take sound, then Slot.onTake's setChanged. The taken stack's
            // onCraftedBy is the caller's (map_crafted).
            self.remove(env, 0, 1);
            self.remove(env, 1, 1);
            self.take_sound("minecraft:ui.cartography_table.take_result");
            self.slot_changed(env, s);
            return;
        }
        if self.slots[s].role == Role::LoomResult {
            // LoomMenu's result onTake: one banner and one dye are used up.
            self.remove(env, 0, 1);
            self.remove(env, 1, 1);
            if self.containers[MAIN][0].is_none() || self.containers[MAIN][1].is_none() {
                self.data[0] = -1;
            }
            self.take_sound("minecraft:ui.loom.take_result");
            return;
        }
        if matches!(self.kind, Kind::CrafterBlock) && self.slots[s].container == MAIN {
            // Slot.onTake -> CrafterSlot.setChanged -> the result refresh.
            self.slot_changed(env, s);
            return;
        }
        if self.slots[s].role != Role::CraftResult {
            return;
        }
        let n = self.grid_size;
        let grid = self.containers[GRID].clone();
        if let Some((id, _)) = env.recipes.find(env.loop_context, &grid, n) {
            self.crafted.push((format!("minecraft:{id}"), grid.clone()));
            self.item_crafted
                .push((format!("minecraft:{id}"), taken.clone()));
        }
        let remaining = env
            .recipes
            .find(env.loop_context, &grid, n)
            .map(|(id, _)| env.recipes.remainders(id, &grid, n))
            .unwrap_or_else(|| {
                // CraftingRecipe.defaultCraftingReminder
                grid.iter()
                    .map(|cell| {
                        cell.as_ref().and_then(|stack| {
                            env.items
                                .get(&stack.item)
                                .and_then(|i| i.crafting_remainder.as_deref())
                                .map(|r| ItemStack::new(r, 1))
                        })
                    })
                    .collect()
            });
        for cell in 0..n * n {
            let mut here = self.containers[GRID][cell].take();
            if let Some(stack) = here.as_mut() {
                stack.count -= 1;
            }
            tidy(&mut here);
            self.containers[GRID][cell] = here;
            if let Some(replacement) = remaining.get(cell).cloned().flatten() {
                match self.containers[GRID][cell].as_mut() {
                    None => self.containers[GRID][cell] = Some(replacement),
                    Some(have) if have.same_item_same_components(&replacement) => {
                        let mut merged = replacement;
                        merged.count += have.count;
                        self.containers[GRID][cell] = Some(merged);
                    }
                    Some(_) => {
                        let mut left = replacement;
                        if !self.inventory_add(env, &mut left) {
                            self.drop_stack(&left);
                        }
                    }
                }
            }
        }
        self.recompute(env);
    }

    fn drop_stack(&mut self, stack: &ItemStack) {
        if stack.count > 0 {
            self.drops.push(crate::item_data::canon_stack(Some(stack)));
            self.dropped.push(stack.clone());
        }
    }

    /// `Slot.tryRemove`.
    fn try_remove(
        &mut self,
        env: &Env,
        s: usize,
        amount: u32,
        max_amount: u32,
    ) -> Option<ItemStack> {
        if !self.may_pickup(env, s) {
            return None;
        }
        if !self.allow_modification(env, s) && max_amount < self.slot(s).map_or(0, |c| c.count) {
            return None;
        }
        let result = self.remove(env, s, amount.min(max_amount))?;
        if self.slot(s).is_none() {
            self.set_by_player_from(env, s, None, Some(&result));
        }
        Some(result)
    }

    fn allow_modification(&self, env: &Env, s: usize) -> bool {
        self.may_pickup(env, s)
            && match self.slot(s) {
                Some(stack) => self.may_place(env, s, stack),
                None => true,
            }
    }

    fn safe_take(
        &mut self,
        env: &Env,
        s: usize,
        amount: u32,
        max_amount: u32,
    ) -> Option<ItemStack> {
        let mut result = self.try_remove(env, s, amount, max_amount)?;
        self.on_take(env, s, &result);
        if self.slots[s].role == Role::CartoResult {
            Self::map_crafted(env.maps, &mut result);
        }
        Some(result)
    }

    /// `Slot.safeInsert`: puts up to `amount` of `input` into the slot.
    fn safe_insert(&mut self, env: &Env, s: usize, input: &mut Option<ItemStack>, amount: u32) {
        let Some(stack) = input.as_mut() else { return };
        if !self.may_place(env, s, stack) {
            return;
        }
        let have = self.slot(s).map_or(0, |c| c.count);
        let max = self.slot_max(env, s, stack);
        let transferable = amount.min(stack.count).min(max.saturating_sub(have));
        if transferable == 0 {
            return;
        }
        match self.slot(s).cloned() {
            None => {
                let part = split(stack, transferable);
                self.set_by_player(env, s, Some(part));
            }
            Some(mut current) if current.same_item_same_components(stack) => {
                stack.count -= transferable;
                current.count += transferable;
                self.set_by_player(env, s, Some(current));
            }
            Some(_) => {}
        }
        tidy(input);
    }

    // ---- Inventory ----

    /// `Inventory.getSlotWithRemainingSpace` (selected slot 0, offhand, rest).
    fn slot_with_space(&self, env: &Env, stack: &ItemStack) -> Option<usize> {
        slot_with_space(&self.containers[PLAYER], self.selected, env.items, stack)
    }

    /// `Inventory.add(stack)`: the leftover stays in `stack`.
    fn inventory_add(&mut self, env: &Env, stack: &mut ItemStack) -> bool {
        inventory_add(
            &mut self.containers[PLAYER],
            self.selected,
            self.creative,
            env.items,
            stack,
        )
    }
}

/// `ItemStack.getMaxStackSize`: the stack's component, else the item's; a removed
/// component (`!minecraft:max_stack_size`) falls back to the item.
pub(crate) fn item_max_stack(items: &Items, stack: &ItemStack) -> u32 {
    let patched = stack.data().and_then(|d| {
        if d.other("!minecraft:max_stack_size").is_some() {
            return None;
        }
        match d.other("minecraft:max_stack_size") {
            Some(horizon_core::nbt::Tag::Int(n)) if (1..=99).contains(n) => Some(*n as u32),
            _ => None,
        }
    });
    patched.unwrap_or_else(|| items.get(&stack.item).map_or(64, |i| i.max_stack))
}

/// Whether a stack can merge in the player's inventory.
fn stackable(items: &Items, stack: &ItemStack) -> bool {
    item_max_stack(items, stack) > 1
        && !(stack.damage > 0 && items.get(&stack.item).is_some_and(|i| i.max_damage > 0))
}

/// `Inventory.getSlotWithRemainingSpace`, preferring selected and offhand slots.
fn slot_with_space(
    inventory: &[Option<ItemStack>],
    selected: usize,
    items: &Items,
    stack: &ItemStack,
) -> Option<usize> {
    [selected, OFFHAND].into_iter().chain(0..36).find(|&i| {
        inventory[i].as_ref().is_some_and(|have| {
            have.same_item_same_components(stack)
                && stackable(items, have)
                && have.count < item_max_stack(items, have)
        })
    })
}

/// `Inventory.add`; leaves any remainder in `stack`.
pub(crate) fn inventory_add(
    inventory: &mut [Option<ItemStack>],
    selected: usize,
    creative: bool,
    items: &Items,
    stack: &mut ItemStack,
) -> bool {
    if stack.count == 0 {
        return false;
    }
    if items.get(&stack.item).is_some_and(|i| i.max_damage > 0) && stack.damage > 0 {
        if let Some(slot) = inventory[..36].iter().position(Option::is_none) {
            inventory[slot] = Some(stack.clone());
            stack.count = 0;
            return true;
        }
        if creative {
            stack.count = 0;
            return true;
        }
        return false;
    }
    loop {
        let last = stack.count;
        let slot = slot_with_space(inventory, selected, items, stack)
            .or_else(|| inventory[..36].iter().position(Option::is_none));
        if let Some(slot) = slot {
            let cell = inventory[slot].get_or_insert_with(|| {
                let mut empty = stack.clone();
                empty.count = 0;
                empty
            });
            let add = stack
                .count
                .min(item_max_stack(items, cell).saturating_sub(cell.count));
            cell.count += add;
            stack.count -= add;
        }
        if stack.count == 0 || stack.count >= last {
            if stack.count == last && creative {
                stack.count = 0;
                return true;
            }
            return stack.count < last;
        }
    }
}

/// `EquipmentSlot` of an item as the inventory menu's quick move sees it.
enum Equip {
    Armor(usize),
    Offhand,
    Other,
}

impl Menu {
    fn equip(env: &Env, stack: &ItemStack) -> Equip {
        match env
            .items
            .get(&stack.item)
            .and_then(|i| i.equip_slot.as_deref())
        {
            Some("feet") => Equip::Armor(0),
            Some("legs") => Equip::Armor(1),
            Some("chest") => Equip::Armor(2),
            Some("head") => Equip::Armor(3),
            Some("offhand") => Equip::Offhand,
            _ => Equip::Other,
        }
    }

    fn has_item(&self, s: usize) -> bool {
        self.slot(s).is_some()
    }

    /// `AbstractContainerMenu.moveItemStackTo`.
    fn move_to(
        &mut self,
        env: &Env,
        stack: &mut ItemStack,
        start: usize,
        end: usize,
        backwards: bool,
    ) -> bool {
        let mut changed = false;
        let order = |_: ()| -> Vec<usize> {
            if backwards {
                (start..end).rev().collect()
            } else {
                (start..end).collect()
            }
        };
        if Self::stackable(env, stack) {
            for dest in order(()) {
                if stack.count == 0 {
                    break;
                }
                // A mod menu's result (unless a mod's hook opens it) and a
                // hidden slot get no top-up either, as a furnace's quick move
                // never aims at its result.
                let refuses = if self.slots[dest].role == Role::Hidden {
                    true
                } else if let Some(id) = self.registry_id() {
                    !apply_menu_slot_rules(
                        env.loop_context,
                        id,
                        self.slots[dest].index,
                        stack,
                        self.slots[dest].role != Role::ModResult,
                        CONTAINER_MAX,
                        Self::max_stack(env, stack),
                    )
                    .0
                } else {
                    false
                };
                let max = match self.slot(dest) {
                    Some(target) if !refuses && target.same_item_same_components(stack) => {
                        self.slot_max(env, dest, target)
                    }
                    _ => continue,
                };
                let slot = self.slots[dest];
                let target = self.containers[slot.container][slot.index]
                    .as_mut()
                    .expect("checked");
                let total = target.count + stack.count;
                if total <= max {
                    stack.count = 0;
                    target.count = total;
                    changed = true;
                    self.slot_changed(env, dest);
                } else if target.count < max {
                    stack.count -= max - target.count;
                    target.count = max;
                    changed = true;
                    self.slot_changed(env, dest);
                }
            }
        }
        if stack.count > 0 {
            for dest in order(()) {
                if !self.has_item(dest) && self.may_place(env, dest, stack) {
                    let max = self.slot_max(env, dest, stack);
                    let part = split(stack, stack.count.min(max));
                    self.set_by_player(env, dest, Some(part));
                    changed = true;
                    break;
                }
            }
        }
        changed
    }

    /// `quickMoveStack` of the menu's kind; the moved stack's original copy
    /// (None when nothing moved).
    fn quick_move(&mut self, env: &Env, index: usize) -> Option<ItemStack> {
        let mut stack = self.slot(index)?.clone();
        let clicked = stack.clone();
        let total = self.slots.len();
        let mut check_count = true;
        let mut after_take = false;
        match self.kind {
            Kind::ItemPicker => {
                // ItemPickerMenu.quickMoveStack: a hotbar stack is deleted.
                if index >= total - 9 {
                    self.put(env, index, None);
                }
                return None;
            }
            Kind::Chest(rows) => {
                check_count = false;
                let n = rows * 9;
                if index < n {
                    if !self.move_to(env, &mut stack, n, total, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, n, false) {
                    return None;
                }
            }
            Kind::Hopper => {
                check_count = false;
                if index < 5 {
                    if !self.move_to(env, &mut stack, 5, 41, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, 5, false) {
                    return None;
                }
            }
            // ChestMenu.quickMoveStack over the mod's slots.
            Kind::Mod(t) => {
                check_count = false;
                let n = t.slots;
                if index < n {
                    if !self.move_to(env, &mut stack, n, total, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, n, false) {
                    return None;
                }
            }
            Kind::Shulker => {
                check_count = false;
                if index < 27 {
                    if !self.move_to(env, &mut stack, 27, total, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, 27, false) {
                    return None;
                }
            }
            Kind::Dispenser => {
                after_take = true;
                if index < 9 {
                    if !self.move_to(env, &mut stack, 9, 45, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, 9, false) {
                    return None;
                }
            }
            Kind::Brewing => {
                after_take = true;
                let slot = self.slots[index];
                let commit = |m: &mut Self, stack: &ItemStack| {
                    m.containers[slot.container][slot.index] =
                        (stack.count > 0).then(|| stack.clone());
                };
                if index <= 4 {
                    if !self.move_to(env, &mut stack, 5, 41, true) {
                        return None;
                    }
                } else if env
                    .items
                    .get(&clicked.item)
                    .is_some_and(|i| i.brewing_fuel.is_some())
                {
                    let moved = self.move_to(env, &mut stack, 4, 5, false);
                    if moved
                        || (env.brewing.is_reagent(&stack)
                            && !self.move_to(env, &mut stack, 3, 4, false))
                    {
                        commit(self, &stack);
                        return None;
                    }
                } else if env.brewing.is_reagent(&stack) {
                    if !self.move_to(env, &mut stack, 3, 4, false) {
                        return None;
                    }
                } else if env.brewing.is_bottle(&stack)
                    || env
                        .items
                        .item_tags
                        .get("minecraft:brewing_potion_inputs")
                        .is_some_and(|m| m.contains(&*stack.item))
                {
                    if !self.move_to(env, &mut stack, 0, 3, false) {
                        return None;
                    }
                } else if (5..32).contains(&index) {
                    if !self.move_to(env, &mut stack, 32, 41, false) {
                        return None;
                    }
                } else if (32..41).contains(&index) {
                    if !self.move_to(env, &mut stack, 5, 32, false) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 5, 41, false) {
                    return None;
                }
            }
            Kind::Loom => {
                after_take = true;
                if index == 3 {
                    if !self.move_to(env, &mut stack, 4, 40, true) {
                        return None;
                    }
                } else if index > 3 {
                    let target = if Self::is_banner(&stack) {
                        Some(0)
                    } else if env.banners.loom_dye(env.items, &stack).is_some() {
                        Some(1)
                    } else if env.banners.is_pattern_item(env.items, &stack) {
                        Some(2)
                    } else {
                        None
                    };
                    match target {
                        Some(t) => {
                            if !self.move_to(env, &mut stack, t, t + 1, false) {
                                return None;
                            }
                        }
                        None if index < 31 => {
                            if !self.move_to(env, &mut stack, 31, 40, false) {
                                return None;
                            }
                        }
                        None => {
                            if index < 40 && !self.move_to(env, &mut stack, 4, 31, false) {
                                return None;
                            }
                        }
                    }
                } else if !self.move_to(env, &mut stack, 4, 40, false) {
                    return None;
                }
            }
            Kind::Stonecutter => {
                after_take = true;
                if index == 1 {
                    if !self.move_to(env, &mut stack, 2, 38, true) {
                        return None;
                    }
                } else if index == 0 {
                    if !self.move_to(env, &mut stack, 2, 38, false) {
                        return None;
                    }
                } else if env
                    .station_recipes
                    .stonecutting
                    .iter()
                    .any(|r| r.input.test_in(env.loop_context, &stack))
                {
                    if !self.move_to(env, &mut stack, 0, 1, false) {
                        return None;
                    }
                } else if (2..29).contains(&index) {
                    if !self.move_to(env, &mut stack, 29, 38, false) {
                        return None;
                    }
                } else if (29..38).contains(&index) && !self.move_to(env, &mut stack, 2, 29, false)
                {
                    return None;
                }
            }
            Kind::Grindstone => {
                after_take = true;
                let full = self.containers[MAIN].iter().all(Option::is_some);
                if index == 2 {
                    if !self.move_to(env, &mut stack, 3, 39, true) {
                        return None;
                    }
                } else if index == 0 || index == 1 {
                    if !self.move_to(env, &mut stack, 3, 39, false) {
                        return None;
                    }
                } else if full {
                    if (3..30).contains(&index) {
                        if !self.move_to(env, &mut stack, 30, 39, false) {
                            return None;
                        }
                    } else if (30..39).contains(&index)
                        && !self.move_to(env, &mut stack, 3, 30, false)
                    {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, 2, false) {
                    return None;
                }
            }
            Kind::Cartography => {
                after_take = true;
                if index == 2 {
                    // onCraftedBy edits the result in its slot first.
                    Self::map_crafted(env.maps, &mut stack);
                    self.containers[RESULT][0] = Some(stack.clone());
                    if !self.move_to(env, &mut stack, 3, 39, true) {
                        return None;
                    }
                } else if index == 0 || index == 1 {
                    if !self.move_to(env, &mut stack, 3, 39, false) {
                        return None;
                    }
                } else if stack.data().is_some_and(|d| d.map_id.is_some()) {
                    if !self.move_to(env, &mut stack, 0, 1, false) {
                        return None;
                    }
                } else if Self::is_carto_extra(&stack) {
                    if !self.move_to(env, &mut stack, 1, 2, false) {
                        return None;
                    }
                } else if (3..30).contains(&index) {
                    if !self.move_to(env, &mut stack, 30, 39, false) {
                        return None;
                    }
                } else if (30..39).contains(&index) && !self.move_to(env, &mut stack, 3, 30, false)
                {
                    return None;
                }
            }
            // LecternMenu.quickMoveStack moves nothing.
            Kind::Lectern => return None,
            Kind::Mount(columns) => {
                // AbstractMountInventoryMenu.quickMoveStack: no count check
                // and no onTake.
                check_count = false;
                let start = 2 + columns * 3;
                if index < start {
                    if !self.move_to(env, &mut stack, start, total, true) {
                        return None;
                    }
                } else if self.may_place(env, 1, &stack) && self.containers[MAIN][1].is_none() {
                    if !self.move_to(env, &mut stack, 1, 2, false) {
                        return None;
                    }
                } else if self.may_place(env, 0, &stack) && self.containers[MAIN][0].is_none() {
                    if !self.move_to(env, &mut stack, 0, 1, false) {
                        return None;
                    }
                } else if columns == 0 || !self.move_to(env, &mut stack, 2, start, false) {
                    // Between the inventory and the hotbar: the stack shrinks
                    // in its slot and EMPTY is returned either way.
                    let (main_end, hotbar_end) = (start + 27, start + 36);
                    if (main_end..hotbar_end).contains(&index) {
                        self.move_to(env, &mut stack, start, main_end, false);
                    } else if (start..main_end).contains(&index) {
                        self.move_to(env, &mut stack, main_end, hotbar_end, false);
                    }
                    let slot = self.slots[index];
                    self.containers[slot.container][slot.index] =
                        (stack.count > 0).then(|| stack.clone());
                    return None;
                }
            }
            Kind::Beacon => {
                // BeaconMenu.quickMoveStack.
                after_take = true;
                if index == 0 {
                    if !self.move_to(env, &mut stack, 1, 37, true) {
                        return None;
                    }
                } else if self.containers[MAIN][0].is_none()
                    && self.may_place(env, 0, &stack)
                    && stack.count == 1
                {
                    if !self.move_to(env, &mut stack, 0, 1, false) {
                        return None;
                    }
                } else if (1..28).contains(&index) {
                    if !self.move_to(env, &mut stack, 28, 37, false) {
                        return None;
                    }
                } else if (28..37).contains(&index) {
                    if !self.move_to(env, &mut stack, 1, 28, false) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 1, 37, false) {
                    return None;
                }
            }
            Kind::Enchanting => {
                after_take = true;
                if index < 2 {
                    if !self.move_to(env, &mut stack, 2, 38, true) {
                        return None;
                    }
                } else if &*stack.item == "minecraft:lapis_lazuli" {
                    if !self.move_to(env, &mut stack, 1, 2, true) {
                        return None;
                    }
                } else {
                    // One item into the empty item slot (setByPlayer).
                    if self.containers[MAIN][0].is_some() || !self.may_place(env, 0, &stack) {
                        return None;
                    }
                    let single = ItemStack {
                        count: 1,
                        ..stack.clone()
                    };
                    stack.count -= 1;
                    // The source slot keeps the rest while the item slot changes.
                    let slot = self.slots[index];
                    self.containers[slot.container][slot.index] =
                        (stack.count > 0).then(|| stack.clone());
                    self.set_by_player(env, 0, Some(single));
                }
            }
            Kind::Anvil | Kind::Smithing => {
                // ItemCombinerMenu.quickMoveStack.
                after_take = true;
                let result = if self.kind == Kind::Anvil { 2 } else { 3 };
                let (start, use_row) = (result + 1, result + 28);
                let end = result + 37;
                let into_inputs = self.kind == Kind::Anvil
                    || (0..3).any(|i| {
                        self.containers[MAIN][i].is_none()
                            && env
                                .station_recipes
                                .smithing_accepts(env.loop_context, i, &stack)
                    });
                if index == result {
                    if !self.move_to(env, &mut stack, start, end, true) {
                        return None;
                    }
                } else if index < result {
                    if !self.move_to(env, &mut stack, start, end, false) {
                        return None;
                    }
                } else if into_inputs && (start..end).contains(&index) {
                    if !self.move_to(env, &mut stack, 0, result, false) {
                        return None;
                    }
                } else if (start..use_row).contains(&index) {
                    if !self.move_to(env, &mut stack, use_row, end, false) {
                        return None;
                    }
                } else if (use_row..end).contains(&index)
                    && !self.move_to(env, &mut stack, start, use_row, false)
                {
                    return None;
                }
            }
            Kind::Merchant => {
                after_take = true;
                if index == 2 {
                    if !self.move_to(env, &mut stack, 3, 39, true) {
                        return None;
                    }
                    // Slot.onQuickCraft uses the original copy and the moved count.
                    self.merchant_remove_count = self
                        .merchant_remove_count
                        .wrapping_add((clicked.count - stack.count) as i32);
                    self.merchant_check_take_achievements(&clicked);
                    if let Some(m) = self.merchant.as_mut() {
                        m.events.push(crate::merchant::Event::TradeSound);
                    }
                } else if index != 0 && index != 1 {
                    if (3..30).contains(&index) {
                        if !self.move_to(env, &mut stack, 30, 39, false) {
                            return None;
                        }
                    } else if (30..39).contains(&index)
                        && !self.move_to(env, &mut stack, 3, 30, false)
                    {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 3, 39, false) {
                    return None;
                }
            }
            Kind::Crafter | Kind::CrafterBlock => {
                after_take = true;
                if index < 9 {
                    if !self.move_to(env, &mut stack, 9, 45, true) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 0, 9, false) {
                    return None;
                }
            }
            Kind::Furnace(recipe) => {
                after_take = true;
                if index == 2 {
                    if !self.move_to(env, &mut stack, 3, 39, true) {
                        return None;
                    }
                    // Slot.onQuickCraft -> FurnaceResultSlot.onQuickCraft:
                    // the moved count, with the stack as it was.
                    let moved = clicked.count - stack.count;
                    if moved > 0 {
                        self.output_removed += moved;
                        self.furnace_take(env, &clicked);
                    }
                } else if index != 1 && index != 0 {
                    let smelts = env.recipes.cooking_accepts(
                        env.loop_context,
                        &format!("minecraft:{recipe}"),
                        &stack,
                    );
                    if smelts {
                        if !self.move_to(env, &mut stack, 0, 1, false) {
                            return None;
                        }
                    } else if Self::is_fuel(env, &stack) {
                        if !self.move_to(env, &mut stack, 1, 2, false) {
                            return None;
                        }
                    } else if (3..30).contains(&index) {
                        if !self.move_to(env, &mut stack, 30, 39, false) {
                            return None;
                        }
                    } else if (30..39).contains(&index)
                        && !self.move_to(env, &mut stack, 3, 30, false)
                    {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 3, 39, false) {
                    return None;
                }
            }
            Kind::Crafting => {
                after_take = true;
                if index == 0 {
                    if !self.move_to(env, &mut stack, 10, 46, true) {
                        return None;
                    }
                } else if (10..46).contains(&index) {
                    if !self.move_to(env, &mut stack, 1, 10, false) {
                        if index < 37 {
                            if !self.move_to(env, &mut stack, 37, 46, false) {
                                return None;
                            }
                        } else if !self.move_to(env, &mut stack, 10, 37, false) {
                            return None;
                        }
                    }
                } else if !self.move_to(env, &mut stack, 10, 46, false) {
                    return None;
                }
            }
            Kind::Inventory => {
                after_take = true;
                let equip = Self::equip(env, &clicked);
                let armor_free = |m: &Self, eq: usize| !m.has_item(8 - eq);
                if index == 0 {
                    if !self.move_to(env, &mut stack, 9, 45, true) {
                        return None;
                    }
                } else if (1..9).contains(&index) {
                    if !self.move_to(env, &mut stack, 9, 45, false) {
                        return None;
                    }
                } else if matches!(equip, Equip::Armor(eq) if armor_free(self, eq)) {
                    let Equip::Armor(eq) = equip else {
                        unreachable!()
                    };
                    let pos = 8 - eq;
                    if !self.move_to(env, &mut stack, pos, pos + 1, false) {
                        return None;
                    }
                } else if matches!(equip, Equip::Offhand) && !self.has_item(45) {
                    if !self.move_to(env, &mut stack, 45, 46, false) {
                        return None;
                    }
                } else if (9..36).contains(&index) {
                    if !self.move_to(env, &mut stack, 36, 45, false) {
                        return None;
                    }
                } else if (36..45).contains(&index) {
                    if !self.move_to(env, &mut stack, 9, 36, false) {
                        return None;
                    }
                } else if !self.move_to(env, &mut stack, 9, 45, false) {
                    return None;
                }
            }
        }
        // The stack was edited in place in its slot.
        let slot = self.slots[index];
        if stack.count == 0 {
            self.containers[slot.container][slot.index] = None;
            // InventoryMenu: setByPlayer(EMPTY, clicked).
            self.set_by_player_from(env, index, None, Some(&clicked));
        } else {
            self.containers[slot.container][slot.index] = Some(stack.clone());
            self.slot_changed(env, index);
        }
        if check_count && stack.count == clicked.count {
            return None;
        }
        if after_take || matches!(self.kind, Kind::Inventory | Kind::Crafting) {
            // Furnace takes pass the remainder; merchant takes pass the original stack.
            let taken = if matches!(self.kind, Kind::Furnace(_)) || self.slots[index].role == Role::MerchantResult {
