use super::*;

impl Game<'_> {
    /// `IItemExtension.damageItem`: `stack` is about to lose `amount` points of
    /// durability (`ItemStack.hurtAndBreak`), after Unbreaking. Change `amount`
    /// (0 or less leaves the item whole) or set `canceled` for no wear.
    /// Server-side.
    pub(in crate::game) fn native_damage_item(
        &mut self,
        stack: &ItemStack,
        amount: &mut i32,
        canceled: &mut bool,
    ) {
        self.fire_damage_item(stack, amount, canceled);
    }
}
