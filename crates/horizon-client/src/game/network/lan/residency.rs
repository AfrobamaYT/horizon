//! Bind actual network members to the existing residency journal.
use super::*;

impl Game<'_> {
    pub(super) fn bind_guest_residency(&self, hub: &mut Hub) {
        hub.guests
            .bind_residency(&self.resident_entities.table.borrow().producer(), |id| {
                crate::game::Dim::from_id(id).key()
            });
    }
}
