use super::*;
impl Game<'_> {
    /// Dropped-item contact after vertical collision response and before block effects.
    pub(in crate::game) fn native_item_after_vertical_collision(
        &mut self,
        call: mod_api::__private::ItemAfterVerticalCollisionCall<'_>,
    ) {
        self.fire_item_after_vertical_collision(call);
    }
}
