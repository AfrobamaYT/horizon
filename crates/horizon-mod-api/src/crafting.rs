//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
use horizon_items::items::Items;
use horizon_world::native_load_context::LoadContext;
use serde_json::Value;
use std::collections::HashSet;
/// A recipe ingredient of a mod's own kind: the entry named a type in a
/// namespace outside `minecraft:`, and the loader kept it for this call.
/// The kind and the JSON the file wrote are here; fill `items` with the
/// item ids the ingredient matches (item ids only: components are not
/// checked here; a match-time hook follows separately). Duplicates are
/// removed keeping first-seen order. The hook may be called more than
/// once per recipe; the answer must be the same. An unclaimed kind
/// leaves the list empty and the recipe fails, as an unknown kind does.
#[inline(never)]
pub fn native_ingredient(
    context: &LoadContext,
    kind: &str,
    data: &serde_json::Map<String, Value>,
    items: &mut Vec<String>,
) {
    crate::dispatch_ingredient(&DataLoadContext::new(context), kind, data, items);
}
/// A mod's own ingredient kind, matching a concrete stack at craft time:
/// the ingredient's `test` calls here with the kind, its JSON and the
/// stack being matched, `accept` already true (the load-time list named
/// the item). Set `accept` to false to refuse the stack — its components
/// are the usual reason. The hook may be called many times a craft; the
/// answer must be the same for the same stack.
#[inline(never)]
pub fn native_ingredient_test(
    kind: &str,
    data: &serde_json::Map<String, Value>,
    stack: &ItemStack,
    accept: &mut bool,
) {
    crate::dispatch_ingredient_test(kind, data, stack, accept);
}
/// Keep first-seen order, drop the rest.
// hidden: a game-side helper, not part of the mod surface
#[doc(hidden)]
pub fn dedup_keep_order(items: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|i| seen.insert(i.clone()));
}
/// A set of accepted items (expanded from an item, list or tag), and the
/// items in its `HolderSet` order (`Ingredient.display`'s `TagSlotDisplay`).
#[derive(Clone, Debug)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct Ingredient {
    /// The items it accepts, however the source shape named them.
    pub items: HashSet<String>,
    /// The same items in `HolderSet` order (`display`).
    pub order: Vec<String>,
    /// A mod's own kind, for matching at craft time: the kind and its
    /// JSON, as the load-time hook saw them (`native_ingredient_test`).
    pub custom: Option<(String, serde_json::Map<String, Value>)>,
    /// NeoForge's shapes, with their children parsed: matching a concrete
    /// stack asks each child its own `test` (a mod's kind among them),
    /// as on NeoForge — the flat list above only names items.
    pub children: Option<(Shape, Vec<Self>)>,
}
/// How a shape's children combine at match time.
#[derive(Clone, Debug)]
// exhaustive: a closed set the game matches without a wildcard
pub enum Shape {
    /// Any child matching the stack is enough.
    Compound,
    /// Every child must match it.
    Intersection,
    /// The first child must, and the second must not.
    Difference,
}
impl Ingredient {
    /// The items of a list, in order.
    pub fn of(items: Vec<String>) -> Self {
        Self {
            items: items.iter().cloned().collect(),
            order: items,
            custom: None,
            children: None,
        }
    }
    pub fn parse(v: &Value, items: &Items, context: &LoadContext) -> Option<Self> {
        let mut list: Vec<String> = Vec::new();
        let mut add = |s: &str| {
            let more: Vec<String> = match s.strip_prefix('#') {
                Some(tag) => items.item_tag_order.get(tag).cloned().unwrap_or_default(),
                None => vec![s.to_owned()],
            };
            for i in more {
                if !list.contains(&i) {
                    list.push(i);
                }
            }
        };
        match v {
            Value::String(s) => add(s),
            Value::Array(a) => a.iter().filter_map(Value::as_str).for_each(&mut add),
            Value::Object(o) => return Self::parse_object(o, items, context),
            _ => return None,
        }
        (!list.is_empty()).then(|| Self::of(list))
    }
    /// NeoForge's ingredient types (`NeoForgeRegistries.Keys.INGREDIENT_TYPES`),
    /// the standard shapes mods' recipes carry, each resolved to the flat item
    /// list it matches. A kind outside these still fails the file: a mod's
    /// own ingredient types come with their own hook, not the loader.
    fn parse_object(
        o: &serde_json::Map<String, Value>,
        items: &Items,
        context: &LoadContext,
    ) -> Option<Self> {
        // Current NeoForge marks a custom ingredient with
        // `neoforge:ingredient_type`; `type` is the older packs' key.
        let kind = o
            .get("neoforge:ingredient_type")
            .or_else(|| o.get("type"))?
            .as_str()?;
        let children = || {
            o.get("children")
                .or_else(|| o.get("ingredients"))
                .and_then(Value::as_array)
        };
        // The shapes' parsed children, for matching at craft time: every
        // arm either fills it or returns.
        let nested;
        let kept = match kind {
            // `CompoundIngredient`: any of its children matching the stack
            // is enough — the union of their items.
            "neoforge:compound" => {
                let subs: Vec<Self> = children()?
                    .iter()
                    .map(|s| Self::parse(s, items, context))
                    .collect::<Option<_>>()?;
                let mut seen = Vec::new();
                for sub in &subs {
                    for item in &sub.order {
                        if !seen.contains(item) {
                            seen.push(item.clone());
                        }
                    }
                }
                nested = Some((Shape::Compound, subs));
                seen
            }
            // `IntersectionIngredient`: every child must match the same
            // stack — the items all of them name.
            "neoforge:intersection" => {
                let subs: Vec<Self> = children()?
                    .iter()
                    .map(|s| Self::parse(s, items, context))
                    .collect::<Option<_>>()?;
                nested = Some((Shape::Intersection, subs.clone()));
                subs.first()?
                    .order
                    .iter()
                    .filter(|i| subs[1..].iter().all(|s| s.items.contains(*i)))
                    .cloned()
                    .collect()
            }
            // The base's items without the subtracted's.
            "neoforge:difference" => {
                let base = Self::parse(o.get("base")?, items, context)?;
                let gone = Self::parse(o.get("subtracted")?, items, context)?;
                nested = Some((Shape::Difference, vec![base.clone(), gone.clone()]));
                base.order
                    .iter()
                    .filter(|i| !gone.items.contains(*i))
                    .cloned()
                    .collect()
            }
            // A mod's own kind: the hook answers with the item list it
            // matches; a bare type name is minecraft's own, not a mod's.
            k if k.contains(':') && !k.starts_with("minecraft:") => {
                let mut list = Vec::new();
                native_ingredient(context, k, o, &mut list);
                dedup_keep_order(&mut list);
                // The kind and its JSON stay with the ingredient: the
                // load-time list only names items, and matching a concrete
                // stack (its components) is the match-time hook's.
                return (!list.is_empty()).then(|| Self {
                    order: list.clone(),
                    items: list.iter().cloned().collect(),
                    custom: Some((k.to_owned(), o.clone())),
                    children: None,
                });
            }
            _ => return None,
        };
        (!kept.is_empty()).then(|| Self {
            items: kept.iter().cloned().collect(),
            order: kept,
            custom: None,
            children: nested,
        })
    }
    /// `Ingredient.test`: a stack the list names — and for a mod's own
    /// kind, one the match-time hook accepts (`ICustomIngredient.test`).
    pub fn test(&self, stack: &ItemStack) -> bool {
        match &self.children {
            // Each child its own `test`, as on NeoForge — a mod's kind
            // among them answers through its hook.
            Some((Shape::Compound, kids)) => kids.iter().any(|k| k.test(stack)),
            Some((Shape::Intersection, kids)) => kids.iter().all(|k| k.test(stack)),
            Some((Shape::Difference, kids)) => {
                kids.first().is_some_and(|k| k.test(stack))
                    && kids.get(1).is_none_or(|k| !k.test(stack))
            }
            None => {
                if !self.items.contains(&*stack.item) {
                    return false;
                }
                if let Some((kind, data)) = &self.custom {
                    let mut accept = true;
                    native_ingredient_test(kind, data, stack, &mut accept);
                    return accept;
                }
                true
            }
        }
    }
}
/// A recipe as its file says, whatever its type (vanilla's or a mod's own): the mod reads it.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct RecipeFile {
    /// `<namespace>:<path>`, its place under `data/<namespace>/recipe/`.
    pub id: String,
    pub json: Arc<Value>,
}
