//! Concrete stack matching, shared by ordinary crafting and placement.
#[cfg(test)]
#[path = "matching_tests.rs"]
mod tests;
use super::*;
impl Ingredient {
    /// `test` with the loop context the game already holds.
    pub fn test_in(
        &self,
        context: &horizon_world::native_load_context::LoopContext,
        stack: &ItemStack,
    ) -> bool {
        self.test_in_guarded(context, stack, &|| true)
            .unwrap_or(false)
    }
    /// Stop before another custom leaf runs if a policy changed the caller's
    /// context. `None` is interruption, never a failed ingredient predicate.
    pub fn test_in_guarded(
        &self,
        context: &horizon_world::native_load_context::LoopContext,
        stack: &ItemStack,
        guard: &impl Fn() -> bool,
    ) -> Option<bool> {
        match &self.children {
            Some((Shape::CustomDisplay, kids)) => match kids.first() {
                Some(base) => base.test_in_guarded(context, stack, guard),
                None => Some(false),
            },
            Some((Shape::Compound, kids)) => {
                for child in kids {
                    if child.test_in_guarded(context, stack, guard)? {
                        return Some(true);
                    }
                }
                Some(false)
            }
            Some((Shape::Intersection, kids)) => {
                for child in kids {
                    if !child.test_in_guarded(context, stack, guard)? {
                        return Some(false);
                    }
                }
                Some(true)
            }
            Some((Shape::Difference, kids)) => {
                let Some(first) = kids.first() else {
                    return Some(false);
                };
                if !first.test_in_guarded(context, stack, guard)? {
                    return Some(false);
                }
                match kids.get(1) {
                    Some(second) => Some(!second.test_in_guarded(context, stack, guard)?),
                    None => Some(true),
                }
            }
            None => {
                if !self.items.contains(&*stack.item) {
                    return Some(false);
                }
                if let Some(IngredientRule::Components {
                    patch,
                    removed,
                    strict,
                }) = &self.rule
                {
                    if stack.count == 0 || &*stack.item == "minecraft:air" {
                        return Some(false);
                    }
                    use horizon_world::component_patch::{ComponentMap, Patch, Prototype};
                    let prototype = Prototype::new(
                        self.component_defaults
                            .get(&*stack.item)
                            .into_iter()
                            .flat_map(|values| values.iter())
                            .map(|(k, v)| (k.clone(), v.clone())),
                    );
                    let mut changes =
                        match crate::item_data::components_tag(stack.damage, stack.data()) {
                            Some(horizon_core::nbt::Tag::Compound(values)) => values,
                            _ => Default::default(),
                        };
                    // This live component is intentionally omitted by the saved-stack codec.
                    if let Some(value) = stack.data().and_then(|data| data.map_post_processing) {
                        changes.insert(
                            "minecraft:map_post_processing".into(),
                            horizon_core::nbt::Tag::Int(i32::from(value)),
                        );
                    }
                    let changes = Patch::new(changes.into_iter().map(|(key, value)| {
                        match key.to_string_lossy().strip_prefix('!') {
                            Some(id) => (id.into(), None),
                            None => (key, Some(value)),
                        }
                    }));
                    let effective = ComponentMap::from_patch(prototype, &changes);
                    // Component codecs compare values, independent of JsonOps' numeric width.
                    let equal = |a: &horizon_core::nbt::Tag, b: &horizon_core::nbt::Tag| {
                        horizon_core::json_nbt::tag_to_json(a)
                            .ok()
                            .zip(horizon_core::json_nbt::tag_to_json(b).ok())
                            .is_some_and(|(a, b)| component_value_equal(&a, &b))
                    };
                    return Some(
                        patch.iter().all(|(key, value)| {
                            effective
                                .get(key)
                                .is_some_and(|actual| equal(value, actual))
                        }) && removed
                            .iter()
                            .all(|id| effective.get(&id.as_str().into()).is_none())
                            && (!strict
                                || effective.entries().all(|(key, _)| patch.contains_key(key))),
                    );
                }
                if let Some((kind, data)) = &self.custom {
                    if !guard() {
                        return None;
                    }
                    let mut accept = true;
                    native_ingredient_test(context, kind, data, stack, &mut accept);
                    if !guard() {
                        return None;
                    }
                    return Some(accept);
                }
                Some(true)
            }
        }
    }
}

// JSON component codecs choose their numeric type; JsonOps/NbtOps widths are not identity.
fn component_value_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
                a == b
            } else if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
                a == b
            } else {
                a.as_f64() == b.as_f64()
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| component_value_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| component_value_equal(a, b)))
        }
        _ => a == b,
    }
}
