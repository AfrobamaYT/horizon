//! Preserve recipe priority while indexing simple recipes and testing non-simple ones.
use super::*;
#[derive(Default)]
struct Kind {
    by_item: crate::fast_hash::FxHashMap<String, Vec<usize>>,
    non_simple: Vec<usize>,
}
pub(super) struct CookIndex(HashMap<String, Kind>);
impl CookIndex {
    pub(super) fn build(recipes: &[Cooking]) -> Self {
        let mut kinds: HashMap<String, Kind> = HashMap::new();
        for (index, recipe) in recipes.iter().enumerate() {
            let kind = kinds.entry(recipe.kind.clone()).or_default();
            if recipe.ingredient.is_simple() {
                for item in &recipe.ingredient.items {
                    kind.by_item.entry(item.clone()).or_default().push(index);
                }
            } else {
                kind.non_simple.push(index);
            }
        }
        Self(kinds)
    }
    pub(super) fn find<'a>(
        &self,
        recipes: &'a [Cooking],
        context: &LoopContext,
        kind: &str,
        input: &ItemStack,
    ) -> Option<&'a Cooking> {
        let kind = self.0.get(kind)?;
        let mut simple = kind
            .by_item
            .get(input.item.as_ref())
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .peekable();
        let mut complex = kind.non_simple.iter().peekable();
        loop {
            let index = match (simple.peek(), complex.peek()) {
                (Some(a), Some(b)) if a < b => simple.next(),
                (_, Some(_)) => complex.next(),
                (Some(_), None) => simple.next(),
                (None, None) => return None,
            }?;
            let recipe = recipes.get(*index)?;
            if recipe.ingredient.test_in(context, input) {
                return Some(recipe);
            }
        }
    }
}
