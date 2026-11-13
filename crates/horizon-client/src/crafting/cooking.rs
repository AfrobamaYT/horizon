//! Indexed cooking recipe lookup.
use super::{CookIndex, Cooking, ItemStack, LoopContext, Recipes};

impl Recipes {
    pub fn cook(&self, context: &LoopContext, kind: &str, input: &ItemStack) -> Option<&Cooking> {
        // Every recipe an item names, not just the first: a mod's match-time
        // hook may refuse the one the first recipe would take (`Ingredient::test`).
        let index = self.cook_index.get_or_init(|| {
            let mut index = CookIndex::new();
            for (i, c) in self.cooking.iter().enumerate() {
                let of_kind = index.entry(c.kind.clone()).or_default();
                for item in &c.ingredient.items {
                    of_kind.entry(item.clone()).or_default().push(i);
                }
            }
            index
        });
        index.get(kind)?.get(&*input.item)?.iter().find_map(|i| {
            self.cooking
                .get(*i)
                .filter(|c| c.ingredient.test_in(context, input))
        })
    }
    /// `cook` by scanning every recipe, as it was before the index.
    #[cfg(test)]
    pub(super) fn cook_scan(&self, context: &LoopContext, kind: &str, input: &ItemStack) -> Option<&Cooking> {
        self.cooking
            .iter()
            .find(|c| c.kind == kind && c.ingredient.test_in(context, input))
    }
}
