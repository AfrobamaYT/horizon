//! `CauldronInteractions`: what an item does to an empty, water, lava or
//! powder snow cauldron. `tools/test_cauldron_parity.py` compares every item
//! with vanilla's `useItemOn`.
use crate::items::{ItemStack, Items};

/// `ItemTags.CAULDRON_CAN_REMOVE_DYE` membership.
pub fn removes_dye(items: &Items, item: &str) -> bool {
    items
        .item_tags
        .get("minecraft:cauldron_can_remove_dye")
        .is_some_and(|members| members.contains(item))
}
use horizon_world::chunk::BlockState;

/// The player side of an interaction.
pub trait Player {
    fn creative(&self) -> bool;
    /// The stack in the interacting hand.
    fn hand(&mut self) -> &mut Option<ItemStack>;
    /// `Inventory.add`; the leftover stays in `stack`.
    fn add(&mut self, stack: &mut ItemStack) -> bool;
    /// `Inventory.contains`.
    fn contains(&self, stack: &ItemStack) -> bool;
    fn drop_stack(&mut self, stack: ItemStack);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// `InteractionResult.SUCCESS`
    Success,
    /// `InteractionResult.CONSUME`
    Consume,
    /// `TRY_WITH_EMPTY_HAND`
    Pass,
}

#[derive(Debug)]
pub struct Interaction {
    pub outcome: Outcome,
    /// The cauldron's new state.
    pub block: Option<BlockState>,
    pub sound: Option<&'static str>,
    /// The custom statistic the player earns (`use_cauldron`, ...).
    pub stat: Option<&'static str>,
    /// `lowerFillLevel` posts BLOCK_CHANGE with the new state.
    pub lower_fill_level: bool,
}

fn state(name: &str, level: Option<u8>) -> BlockState {
    BlockState {
        name: name.to_owned(),
        properties: level
            .map(|l| ("level".to_owned(), l.to_string()))
            .into_iter()
            .collect(),
    }
}

fn pass() -> Interaction {
    Interaction {
        outcome: Outcome::Pass,
        block: None,
        sound: None,
        stat: None,
        lower_fill_level: false,
    }
}

/// `ItemUtils.createFilledResult`: `new` for one item of the stack in `player`'s hand. In
/// creative (with `limit_creative`) the hand is kept and `new` added unless the inventory has
/// it; otherwise one item of the hand is used up, and `new` takes its place when it was the
/// last, else goes into the inventory, else is dropped.
pub fn create_filled_result(player: &mut dyn Player, new: ItemStack, limit_creative: bool) {
    let creative = player.creative();
    if limit_creative && creative {
        if !player.contains(&new) {
            let mut new = new;
            player.add(&mut new);
        }
        return;
    }
    let hand = player.hand();
    if !creative && let Some(held) = hand.as_mut() {
        held.count -= 1;
        if held.count == 0 {
            *hand = None;
        }
    }
    if hand.is_none() {
        *hand = Some(new);
        return;
    }
    let mut new = new;
    if !player.add(&mut new) {
        player.drop_stack(new);
    }
}

/// `LayeredCauldronBlock.handleEntityOnFireInside`: a burning entity in a
/// water cauldron lowers it a level; a powder snow cauldron becomes a water
/// cauldron of its level first, then lowers.
pub fn extinguished(cauldron: &BlockState) -> BlockState {
    if cauldron.name == "minecraft:powder_snow_cauldron" {
        let mut water = cauldron.clone();
        water.name = "minecraft:water_cauldron".into();
        lowered(&water)
    } else {
        lowered(cauldron)
    }
}

/// `LayeredCauldronBlock.lowerFillLevel`.
fn lowered(cauldron: &BlockState) -> BlockState {
    let level: u8 = cauldron
        .properties
        .get("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(1);
    if level <= 1 {
        state("minecraft:cauldron", None)
    } else {
        let mut next = cauldron.clone();
        next.properties
            .insert("level".into(), (level - 1).to_string());
        next
    }
}

fn changed(block: BlockState, sound: Option<&'static str>, stat: &'static str) -> Interaction {
    Interaction {
        outcome: Outcome::Success,
        block: Some(block),
        sound,
        stat: Some(stat),
        lower_fill_level: false,
    }
}

fn changed_lowered(
    cauldron: &BlockState,
    sound: Option<&'static str>,
    stat: &'static str,
) -> Interaction {
    let mut result = changed(lowered(cauldron), sound, stat);
    result.lower_fill_level = true;
    result
}

const COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

#[derive(Clone, Copy)]
enum Action {
    EmptyLava,
    EmptyWater,
    EmptySnow,
    FillWater,
    FillLava,
    FillSnow,
    EmptyPotion,
    FillBottle,
    TopUpPotion,
    CleanBanner,
    CleanShulker,
}

/// Vanilla's per-cauldron item dispatch, built once and shared by all views.
fn interactions() -> &'static [std::collections::HashMap<String, Action>; 4] {
    use Action::*;
    use std::sync::OnceLock;
    static TABLE: OnceLock<[std::collections::HashMap<String, Action>; 4]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut maps = std::array::from_fn::<_, 4, _>(|_| std::collections::HashMap::new());
        for map in &mut maps {
            for (item, action) in [
                ("lava_bucket", EmptyLava),
                ("water_bucket", EmptyWater),
                ("powder_snow_bucket", EmptySnow),
            ] {
                map.insert(format!("minecraft:{item}"), action);
            }
        }
        for (kind, item, action) in [
            (0, "potion", EmptyPotion),
            (1, "bucket", FillWater),
            (1, "glass_bottle", FillBottle),
            (1, "potion", TopUpPotion),
            (2, "bucket", FillLava),
            (3, "bucket", FillSnow),
        ] {
            maps[kind].insert(format!("minecraft:{item}"), action);
        }
        for color in COLORS {
            maps[1].insert(format!("minecraft:{color}_banner"), CleanBanner);
            maps[1].insert(format!("minecraft:{color}_shulker_box"), CleanShulker);
        }
        maps
    })
}

/// The cauldron interaction of the held item (`useItemOn`); the hand stack is
/// changed through `player`. `removes_dye` is whether the held item is in the
/// `cauldron_can_remove_dye` tag.
pub fn interact(
    cauldron: &BlockState,
    under_water: bool,
    removes_dye: bool,
    player: &mut dyn Player,
) -> Interaction {
    let Some(held) = player.hand().clone() else {
        return pass();
    };
    let kind = match cauldron.name.as_str() {
        "minecraft:cauldron" => 0,
        "minecraft:water_cauldron" => 1,
        "minecraft:lava_cauldron" => 2,
        "minecraft:powder_snow_cauldron" => 3,
        _ => return pass(),
    };
    let level: u8 = cauldron
        .properties
        .get("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);
    let item = &*held.item;
    let water_potion = held.data().and_then(|d| d.potion.as_deref()) == Some("minecraft:water");
    // Dispatcher.get: tags first, then the item map.
    if kind == 1 && removes_dye {
        if held.data().and_then(|d| d.dyed_color).is_none() {
            return pass();
        }
        if let Some(stack) = player.hand().as_mut() {
            stack.data_mut().dyed_color = None;
            stack.tidy();
        }
        return changed_lowered(cauldron, None, "clean_armor");
    }
    let fill_bucket = |player: &mut dyn Player, new: &str, can_fill: bool, sound| {
        if !can_fill {
            return pass();
        }
        create_filled_result(player, ItemStack::new(new, 1), true);
        changed(
            state("minecraft:cauldron", None),
            Some(sound),
            "use_cauldron",
        )
    };
    let empty_bucket = |player: &mut dyn Player, block: BlockState, sound| {
        create_filled_result(player, ItemStack::new("minecraft:bucket", 1), true);
        changed(block, Some(sound), "fill_cauldron")
    };
    let Some(action) = interactions()[kind].get(item) else {
        return pass();
    };
    match action {
        Action::EmptyLava => {
            if under_water {
                Interaction {
                    outcome: Outcome::Consume,
                    block: None,
                    sound: None,
                    stat: None,
                    lower_fill_level: false,
                }
            } else {
                empty_bucket(
                    player,
                    state("minecraft:lava_cauldron", None),
                    "minecraft:item.bucket.empty_lava",
                )
            }
        }
        Action::EmptyWater => empty_bucket(
            player,
            state("minecraft:water_cauldron", Some(3)),
            "minecraft:item.bucket.empty",
        ),
        Action::EmptySnow => {
            if under_water {
                Interaction {
                    outcome: Outcome::Consume,
                    block: None,
                    sound: None,
                    stat: None,
                    lower_fill_level: false,
                }
            } else {
                empty_bucket(
                    player,
                    state("minecraft:powder_snow_cauldron", Some(3)),
                    "minecraft:item.bucket.empty_powder_snow",
                )
            }
        }
        Action::EmptyPotion => {
            if !water_potion {
                return pass();
            }
            create_filled_result(player, ItemStack::new("minecraft:glass_bottle", 1), true);
            changed(
                state("minecraft:water_cauldron", Some(1)),
                Some("minecraft:item.bottle.empty"),
                "use_cauldron",
            )
        }
        Action::FillWater => fill_bucket(
            player,
            "minecraft:water_bucket",
            level == 3,
            "minecraft:item.bucket.fill",
        ),
        Action::FillBottle => {
            create_filled_result(
                player,
                ItemStack::new("minecraft:potion", 1).with_potion("minecraft:water"),
                true,
            );
            changed_lowered(cauldron, Some("minecraft:item.bottle.fill"), "use_cauldron")
        }
        Action::TopUpPotion => {
            if level == 3 || !water_potion {
                return pass();
            }
            create_filled_result(player, ItemStack::new("minecraft:glass_bottle", 1), true);
            let mut next = cauldron.clone();
            // BlockState.cycle on a 1..=3 property
            next.properties
                .insert("level".into(), (level % 3 + 1).to_string());
            changed(next, Some("minecraft:item.bottle.empty"), "use_cauldron")
        }
        Action::CleanBanner => {
            let layers = held.data().map_or(0, |d| d.banner_patterns.len());
            if layers == 0 {
                return pass();
            }
            let mut cleaned = held.clone();
            cleaned.count = 1;
            cleaned.data_mut().banner_patterns.pop();
            cleaned.tidy();
