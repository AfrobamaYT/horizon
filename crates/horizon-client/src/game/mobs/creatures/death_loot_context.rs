use super::*;

impl Game<'_> {
    /// The actual killing hand, copied before loot callbacks can change inventories.
    pub(super) fn death_killer_hand(
        &mut self,
        m: &Mob,
        killer: Option<Option<u32>>,
    ) -> Option<ItemStack> {
        if !mod_api::__private::listened(mod_api::HookId::LootDrops) {
            return None;
        }
        match m.last_damage.and_then(|(d, _)| d.attacker) {
            Some(horizon_client::mobs::Who::Player) => match killer {
                Some(Some(conn)) if self.lan_acting != Some(conn) => {
                    self.as_guest(conn, |g| g.held().cloned()).flatten()
                }
                Some(_) => self.held().cloned(),
                None => None,
            },
            Some(horizon_client::mobs::Who::Mob(id)) => self
                .mobs
                .iter()
                .find(|mob| mob.id == id)
                .and_then(|mob| mob.extra.main_hand_stack())
                .cloned(),
            None => None,
        }
    }
}
