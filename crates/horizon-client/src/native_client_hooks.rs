//! Callbacks into the executable for native trait mods' client-library hooks.
use crate::items::ItemStack;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Client-library hooks, installed before any menu, entity or recipe is used.
pub struct NativeClientHooks {
    pub menu_slot_rules: fn(&str, usize, &ItemStack, &mut bool, &mut u32),
    pub living_breathe: fn(u32, &str, &mut bool, &mut i32, &mut i32),
    pub has_mods: fn() -> bool,
    pub player_name_format: fn(&crate::text::Text, &mut crate::text::Text),
    pub player_tab_list_name_format: fn(u128, &str, &mut Option<crate::text::Text>),
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

pub(crate) fn has_mods() -> bool {
    HOOKS.get().is_some_and(|hooks| (hooks.has_mods)())
}

pub(crate) fn player_name_format(username: &crate::text::Text, displayname: &mut crate::text::Text) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_name_format)(username, displayname);
    }
}

pub(crate) fn player_tab_list_name_format(
    player: u128,
    name: &str,
    display_name: &mut Option<crate::text::Text>,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_tab_list_name_format)(player, name, display_name);
    }
}

pub(crate) fn ingredient(kind: &str, data: &Map<String, Value>, items: &mut Vec<String>) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.ingredient)(kind, data, items);
    }
}
