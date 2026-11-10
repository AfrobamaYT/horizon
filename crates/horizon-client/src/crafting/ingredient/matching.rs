//! Concrete stack matching, shared by ordinary crafting and placement.
use super::*;
impl Ingredient {
    /// `test` with the loop context the game already holds, for the game's
    /// own matching.
    pub fn test_in(
        &self,
        context: &horizon_world::native_load_context::LoopContext,
        stack: &ItemStack,
    ) -> bool {
        match &self.children {
            // Each child its own `test`, as on NeoForge — a mod's kind
            // among them answers through its hook.
            Some((Shape::Compound, kids)) => kids.iter().any(|k| k.test_in(context, stack)),
            Some((Shape::Intersection, kids)) => kids.iter().all(|k| k.test_in(context, stack)),
            Some((Shape::Difference, kids)) => {
                kids.first().is_some_and(|k| k.test_in(context, stack))
                    && kids.get(1).is_none_or(|k| !k.test_in(context, stack))
            }
            None => {
                if !self.items.contains(&*stack.item) {
                    return false;
                }
                if let Some((kind, data)) = &self.custom {
                    let mut accept = true;
                    native_ingredient_test(context, kind, data, stack, &mut accept);
                    return accept;
                }
                true
            }
        }
    }
}
