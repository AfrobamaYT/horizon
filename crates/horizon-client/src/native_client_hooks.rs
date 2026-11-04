//! Callbacks into the executable for native trait mods' client-library hooks.
use crate::items::ItemStack;
use horizon_world::native_load_context::LoopContext;
use std::sync::OnceLock;

/// Client-library hooks, installed before any menu, entity or recipe is used.
pub struct NativeClientHooks {
    pub menu_slot_rules: fn(&LoopContext, &str, usize, &ItemStack, &mut bool, &mut u32),
    pub item_stacked_on_other: fn(&LoopContext, &mut crate::menu::StackedOnOther<'_>, &mut bool),
    pub living_heal: fn(&LoopContext, u32, &mut f32, &mut bool),
    pub living_breathe: fn(&LoopContext, u32, &str, &mut bool, &mut i32, &mut i32),
    pub farmland_trample: fn(u32, [i32; 3], u32, f64, &mut bool),
    pub has_mods: fn() -> bool,
    pub player_name_format: fn(&LoopContext, &crate::text::Text, &mut crate::text::Text),
    pub player_tab_list_name_format: fn(&LoopContext, u128, &str, &mut Option<crate::text::Text>),
    pub living_drown: fn(&LoopContext, u32, &str, &mut bool, &mut f32, i32, &mut bool),
    pub living_fall: fn(u32, &str, &mut f64, &mut f32, &mut bool),
    pub living_jump: fn(u32, &str),
    pub living_jump_power: fn(&LoopContext, u32, &str, &mut f32),
    pub living_speed: fn(&LoopContext, u32, &str, &mut f32),
    pub living_flying_speed: fn(&LoopContext, u32, &str, &mut f32),
    pub player_stays_on_ground_surface: fn(u32, &mut bool),
    pub ender_entity_teleport: fn(u32, &str, [f64; 3], &mut [f64; 3], &mut bool),
}

static HOOKS: OnceLock<NativeClientHooks> = OnceLock::new();

/// Install the process-wide callbacks once; a second installation keeps the first and answers false.
pub fn install(hooks: NativeClientHooks) -> bool {
    HOOKS.set(hooks).is_ok()
}

pub(crate) fn menu_slot_rules(
    context: &LoopContext,
    menu: &str,
    slot: usize,
    stack: &ItemStack,
    may_place: &mut bool,
    max_stack: &mut u32,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.menu_slot_rules)(context, menu, slot, stack, may_place, max_stack);
    }
}

pub(crate) fn item_stacked_on_other(
    context: &LoopContext,
    event: &mut crate::menu::StackedOnOther<'_>,
    canceled: &mut bool,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.item_stacked_on_other)(context, event, canceled);
    }
}

pub(crate) fn living_heal(context: &LoopContext, entity: u32, amount: &mut f32, cancel: &mut bool) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_heal)(context, entity, amount, cancel);
    }
}

pub(crate) fn living_breathe(
    context: &LoopContext,
    entity: u32,
    kind: &str,
    can_breathe: &mut bool,
    consume: &mut i32,
    refill: &mut i32,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_breathe)(context, entity, kind, can_breathe, consume, refill);
    }
}

pub(crate) fn farmland_trample(
    entity: u32,
    pos: [i32; 3],
    state: u32,
    distance: f64,
    cancel: &mut bool,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.farmland_trample)(entity, pos, state, distance, cancel);
    }
}

pub(crate) fn has_mods() -> bool {
    HOOKS.get().is_some_and(|hooks| (hooks.has_mods)())
}

pub(crate) fn player_name_format(
    context: &LoopContext,
    username: &crate::text::Text,
    displayname: &mut crate::text::Text,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_name_format)(context, username, displayname);
    }
}

pub(crate) fn player_tab_list_name_format(
    context: &LoopContext,
    player: u128,
    name: &str,
    display_name: &mut Option<crate::text::Text>,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_tab_list_name_format)(context, player, name, display_name);
    }
}

pub(crate) fn living_drown(
    context: &LoopContext,
    entity: u32,
    kind: &str,
    is_drowning: &mut bool,
    damage_amount: &mut f32,
    bubble_count: i32,
    cancel: &mut bool,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_drown)(
            context,
            entity,
            kind,
            is_drowning,
            damage_amount,
            bubble_count,
            cancel,
        );
    }
}

pub(crate) fn living_fall(
    entity: u32,
    kind: &str,
    distance: &mut f64,
    multiplier: &mut f32,
    cancel: &mut bool,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_fall)(entity, kind, distance, multiplier, cancel);
    }
}

pub(crate) fn living_jump(entity: u32, kind: &str) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_jump)(entity, kind);
    }
}

pub(crate) fn living_jump_power(context: &LoopContext, entity: u32, kind: &str, power: &mut f32) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_jump_power)(context, entity, kind, power);
    }
}

pub(crate) fn living_speed(context: &LoopContext, entity: u32, kind: &str, speed: &mut f32) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_speed)(context, entity, kind, speed);
    }
}

pub(crate) fn living_flying_speed(context: &LoopContext, entity: u32, kind: &str, speed: &mut f32) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.living_flying_speed)(context, entity, kind, speed);
    }
}

pub(crate) fn player_stays_on_ground_surface(entity: u32, staying: &mut bool) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.player_stays_on_ground_surface)(entity, staying);
    }
}

pub(crate) fn ender_entity_teleport(
    entity: u32,
    kind: &str,
    from: [f64; 3],
    target: &mut [f64; 3],
    cancel: &mut bool,
) {
    if let Some(hooks) = HOOKS.get() {
        (hooks.ender_entity_teleport)(entity, kind, from, target, cancel);
    }
}
