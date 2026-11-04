//! The enchantments gameplay sees on a stack (`ItemStack.getAllEnchantments`
//! and `getEnchantmentLevel` as NeoForge has them): horizon-items' level
//! reads ask here through `native_enchantment_hooks`, and this asks the mods'
//! `get_enchantment_level` (`GetEnchantmentLevelEvent`).
use horizon_items::item_data::Levels;
use horizon_items::items::ItemStack;
use mod_api::Identifier;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    /// Each enchantment id the game has handed a mod, parsed once (`None`
    /// for one the API can't name). The registry bounds it.
    static IDS: RefCell<HashMap<Arc<str>, Option<Identifier>>> = RefCell::default();
    /// The list handed to the mods, kept between reads so a read whose
    /// levels no mod changes allocates nothing.
    static SPARE: Cell<Vec<(Identifier, u32)>> = const { Cell::new(Vec::new()) };
}

/// The callback horizon-items calls (installed in `entry.rs`): `own` as the
/// mods leave it, or `None` when no mod listens or none changes it.
pub fn get_enchantment_level(
    stack: &ItemStack,
    target: Option<&str>,
    own: &[(Arc<str>, u32)],
) -> Option<Levels> {
    if !mod_api::__private::listened(mod_api::HookId::GetEnchantmentLevel) {
        return None;
    }
    let target = target.and_then(identifier);
    let mut entries = SPARE.take();
    entries.clear();
    // An id the API can't name is kept as it is, ahead of the rest.
    let mut kept = Vec::new();
    for (id, level) in own {
        match identifier(id) {
            Some(parsed) => entries.push((parsed, *level)),
            None => kept.push((id.clone(), *level)),
        }
    }
    let mut enchantments = mod_api::__private::game::item_enchantments(entries);
    mod_api::enchantments::native_get_enchantment_level(stack, target.as_ref(), &mut enchantments);
    let (mut entries, changed) = mod_api::__private::game::item_enchantments_entries(enchantments);
    let levels = changed.then(|| {
        kept.extend(entries.iter().map(|(id, level)| (text(own, id), *level)));
        kept
    });
    entries.clear();
    SPARE.set(entries);
    levels
}

/// `id` parsed, from the cache after the first time.
fn identifier(id: &str) -> Option<Identifier> {
    IDS.with_borrow_mut(|ids| {
        if let Some(parsed) = ids.get(id) {
            return parsed.clone();
        }
        let parsed = Identifier::parse_namespaced(id);
        ids.insert(Arc::from(id), parsed.clone());
        parsed
    })
}

/// `id` as the game's own text: the stack's when it had it already.
fn text(own: &[(Arc<str>, u32)], id: &Identifier) -> Arc<str> {
    own.iter()
        .find(|(text, _)| text.split_once(':') == Some((id.namespace(), id.path())))
        .map_or_else(|| Arc::from(id.to_string()), |(text, _)| text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::{GetEnchantmentLevelEvent, Hooks, ModEntry, testing::with_mods};
    use std::borrow::Cow;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    /// An iron sword fights as if it had Looting II and no Sharpness.
    struct Keen;
    #[mod_api::__subscriptions]
    impl Hooks for Keen {
        fn get_enchantment_level(&self, event: &mut GetEnchantmentLevelEvent<'_>) {
            if &*event.stack.item != "minecraft:iron_sword" {
                return;
            }
            let looting = id("minecraft:looting");
            if event.target.is_none_or(|target| *target == looting) {
                event.enchantments.upgrade(looting, 2);
            }
            event.enchantments.set(id("minecraft:sharpness"), 0);
        }
    }

    /// Gameplay reads see the mod's levels; the stack's own component, a
    /// stack the mod leaves alone and every stack with no mod listening
    /// keep theirs, borrowed.
    #[test]
    fn a_mod_changes_the_levels_gameplay_reads() {
        let _ = horizon_items::native_enchantment_hooks::install(get_enchantment_level);
        let mut iron = ItemStack::new("minecraft:iron_sword", 1);
        iron.enchant("minecraft:sharpness", 3);
        let mut diamond = ItemStack::new("minecraft:diamond_sword", 1);
        diamond.enchant("minecraft:sharpness", 3);
        with_mods(Vec::new().into(), || {
            assert!(matches!(iron.all_enchantments(), Cow::Borrowed(_)));
            assert_eq!(iron.enchantment_level("minecraft:sharpness"), 3);
        });
        with_mods(vec![ModEntry::new("keen", Box::new(Keen))].into(), || {
            assert_eq!(
                &*iron.all_enchantments(),
                [(Arc::from("minecraft:looting"), 2)]
            );
            assert_eq!(iron.enchantment_level("minecraft:looting"), 2);
            assert_eq!(iron.enchantment_level("minecraft:sharpness"), 0);
            assert_eq!(iron.tag_enchantment_level("minecraft:sharpness"), 3);
            assert_eq!(iron.enchantments(), [(Arc::from("minecraft:sharpness"), 3)]);
            assert!(matches!(diamond.all_enchantments(), Cow::Borrowed(_)));
            assert_eq!(diamond.enchantment_level("minecraft:sharpness"), 3);
        });
    }

    /// `ItemEnchantments` edits as `ItemEnchantments.Mutable` does, and
    /// knows whether anything changed.
    #[test]
    fn item_enchantments_edit_as_neoforges_mutable() {
        use mod_api::__private::game::{item_enchantments, item_enchantments_entries};
        let unchanged = |e: mod_api::ItemEnchantments| !item_enchantments_entries(e).1;
        let mut e = item_enchantments(vec![(id("minecraft:sharpness"), 3)]);
        e.upgrade(id("minecraft:sharpness"), 2);
        e.set(id("minecraft:sharpness"), 3);
        e.set(id("minecraft:looting"), 0);
        assert!(!e.retain(|_, _| true));
        assert!(unchanged(e));
        let mut e = item_enchantments(vec![(id("minecraft:sharpness"), 3)]);
        e.upgrade(id("minecraft:sharpness"), 4);
        e.set(id("minecraft:looting"), 1000);
        assert_eq!(e.level(&id("minecraft:looting")), 255);
        e.set(id("minecraft:sharpness"), 0);
        assert_eq!(e.len(), 1);
        assert!(e.retain(|id, _| id.path() != "looting"));
        assert!(e.is_empty());
        assert_eq!(item_enchantments_entries(e), (Vec::new(), true));
    }

    #[test]
    fn levels_keep_the_games_ids() {
        let own: Levels = vec![(Arc::from("minecraft:sharpness"), 3)];
        assert!(Arc::ptr_eq(
            &text(&own, &id("minecraft:sharpness")),
            &own[0].0
        ));
        assert_eq!(&*text(&own, &id("minecraft:looting")), "minecraft:looting");
        assert_eq!(
            identifier("minecraft:sharpness"),
            Some(id("minecraft:sharpness"))
        );
        assert_eq!(identifier("Not An Id"), None);
    }
}
