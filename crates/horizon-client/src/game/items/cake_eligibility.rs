use super::*;

impl GameLevel<'_> {
    /// Cake reaches this only when it will try to eat, not when adding or
    /// extinguishing a candle or when using an unrelated block.
    pub(super) fn cake_eligibility(&self, vanilla: bool) -> bool {
        if !mod_api::__private::listened(mod_api::HookId::CanEat) {
            return vanilla;
        }
        use crate::game::api_values::TryIntoApi;
        let Ok(stack) = self.held_stack.try_into_api() else {
            return vanilla;
        };
        let mut answer = None;
        mod_api::dispatch_can_eat(self, stack.as_ref(), vanilla, &mut answer);
        answer.unwrap_or(vanilla)
    }
}
