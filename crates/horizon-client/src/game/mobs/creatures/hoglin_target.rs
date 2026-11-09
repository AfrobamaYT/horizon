//! Apply a hoglin broadcast after its vanilla target selection and mod approval.
use super::{Game, Vec3, Who};

impl Game<'_> {
    pub(super) fn hoglin_attack_target(&mut self, mob: u32, target: Who) {
        let places: Vec<(u32, Vec3)> = self
            .mobs
            .iter()
            .filter(|m| m.alive())
            .map(|m| (m.id, m.position))
            .collect();
        let position = |who: Who| match who {
            Who::Mob(id) => places.iter().find(|(i, _)| *i == id).map(|(_, p)| *p),
            Who::Player => None,
        };
        let proposal =
            self.mobs.iter().find(|m| m.id == mob).and_then(|m| {
                horizon_client::mobs::piglin::receive_attack_target(m, target, position)
            });
        if let Some(target) = proposal {
            let player = self
                .lan
                .as_ref()
                .and_then(|host| host.mob_players.get(&mob).copied())
                .or(self.lan_acting);
            self.set_event_ai_target(mob, Some(target), player);
        }
    }
}
