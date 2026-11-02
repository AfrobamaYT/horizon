//! Callbacks into the executable for native trait mods' client-library hooks.
use crate::items::ItemStack;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Client-library hooks, installed before any menu, entity or recipe is used.
pub struct NativeClientHooks {
    pub menu_slot_rules: fn(&str, usize, &ItemStack, &mut bool, &mut u32),
    pub item_stacked_on_other: fn(&mut crate::menu::StackedOnOther<'_>, &mut bool),
    pub living_breathe: fn(u32, &str, &mut bool, &mut i32, &mut i32),
    pub player_flyable_fall: fn(u32, f32, f32, &mut Option<crate::landing::PlayerFlyableFall>),
    pub ingredient: fn(&str, &Map<String, Value>, &mut Vec<String>),
}

static HOOKS: OnceLock<NativeClientHooks> = OnceLock::new();

/// Install the process-wide callbacks once; a second installation keeps the first.
pub fn install(hooks: NativeClientHooks) -> Result<(), NativeClientHooks> {
    HOOKS.set(hooks)
}

pub(crate) fn menu_slot_rules(
    menu: &str,
    slot: usize,
    stack: &ItemStack,
    may_place: &mut bool,
    max_stack: &mut u32,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.menu_slot_rules)(menu, slot, stack, may_place, max_stack);
    }
}

pub(crate) fn item_stacked_on_other(event: &mut crate::menu::StackedOnOther<'_>, canceled: &mut bool) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.item_stacked_on_other)(event, canceled);
    }
}

pub(crate) fn living_breathe(
    entity: u32,
    kind: &str,
    can_breathe: &mut bool,
    consume: &mut i32,
    refill: &mut i32,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_breathe)(entity, kind, can_breathe, consume, refill);
    }
}

pub(crate) fn player_flyable_fall(
    player: u32,
    distance: f32,
    multiplier: f32,
    replacement: &mut Option<crate::landing::PlayerFlyableFall>,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_flyable_fall)(player, distance, multiplier, replacement);
    }
}

pub(crate) fn ingredient(kind: &str, data: &Map<String, Value>, items: &mut Vec<String>) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.ingredient)(kind, data, items);
    }
}
