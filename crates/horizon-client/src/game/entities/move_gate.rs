//! The game layer's door of the move gate (#2414): who is moving, and the one
//! `Game::entity_move` the pushers and family ticks arm a move through. PR 1a
//! carries no hook — every decision is `Run` and the tick cost is one branch;
//! PR 1b fires the `entity_move` hook inside the client `move_gate::arm` this
//! door calls, with a revision recheck before the kind's physics runs.
use super::Game;
use super::entity::{Entity, EntityHandle, with_entity};
use horizon_core::geometry::Vec3;
use horizon_world::entity_move::MoverType;
use horizon_world::move_gate::MoveGate;

/// Which body a move is for: the local player window, a LAN guest, or a listed
/// entity. The piston and shulker-box pushes and the family ticks name their
/// mover with it.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(in crate::game) enum MoveWho {
    Player,
    Guest(u32),
    Entity(EntityHandle),
}

impl Game<'_> {
    /// Arm one physical `Entity.move` invocation (#2414) for `who` with `cause`
    /// and hand back its gate: the caller then runs the kind's physics with
    /// the movement the gate decided (`delta`), and skips it when the gate
    /// says so. A pusher arms before its own restrictions — the piston limit
    /// included — so a cancelled move spends none of them.
    pub(in crate::game) fn entity_move(
        &mut self,
        who: MoveWho,
        cause: MoverType,
        delta: Vec3,
    ) -> MoveGate {
        let (entity, kind) = self.move_who_identity(who);
        horizon_client::move_gate::armed(&self.loop_context, entity, &kind, cause, delta)
    }

    /// The entity id and kind name (`minecraft:zombie`) a move is for; the
    /// player window's own id and name for it.
    fn move_who_identity(&self, who: MoveWho) -> (u32, String) {
        match who {
            MoveWho::Player => (
                self.player.as_ref().map_or(0, |p| p.entity_id as u32),
                "minecraft:player".to_owned(),
            ),
            MoveWho::Guest(conn) => self.lan.as_ref().map_or((0, String::new()), |host| {
                host.hub.guest(conn).map_or((0, String::new()), |g| {
                    (g.entity_id as u32, "minecraft:player".to_owned())
                })
            }),
            MoveWho::Entity(handle) => with_entity!(self, handle, entity => {
                (entity.id(), entity.native_type().into_owned())
            }),
        }
    }
}
