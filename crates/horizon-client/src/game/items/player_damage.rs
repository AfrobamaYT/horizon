use super::api_values::TryIntoApi;
use super::{Game, answered_wear, armor_slot, durability_damage};
use horizon_client::{items::ItemStack, motion::MotionEvent};

impl Game<'_> {
    /// `Player.hurtHelmet`: the head slot alone wears.
    pub(in crate::game) fn hurt_helmet(
        &mut self,
        source: &horizon_client::health::HurtSource<'_>,
        damage: f32,
    ) {
        self.hurt_equipment(source, damage, &[0]);
    }
}
