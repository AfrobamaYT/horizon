//! Dragon-owned identity and attachment metadata.
use super::*;
pub(in crate::game) struct DragonResidency {
    pub(super) lifetime: horizon_mod_runtime::attachments::AttachmentLifetime,
    pub(in crate::game) part_uuids: [[i32; 4]; 8],
}
impl Default for DragonResidency {
    fn default() -> Self {
        Self {
            lifetime: Default::default(),
            // Independent entity streams, never the level or dragon's ongoing RNG.
            part_uuids: std::array::from_fn(|_| {
                horizon_client::save::insecure_uuid(&mut random_source::entity())
            }),
        }
    }
}
impl horizon_mod_runtime::attachments::HasAttachmentLifetime for Dragon {
    fn attachment_lifetime(&mut self) -> &mut horizon_mod_runtime::attachments::AttachmentLifetime {
        &mut self.residency.lifetime
    }
}

impl Game<'_> {
    /// The dragon the level holds and the index (`getSubEntities` order) of
    /// its part with that entity id, while the dragon is there, dying or
    /// not: a part is an entity for as long as its parent is.
    pub(in crate::game) fn resident_dragon_part_source(&self, id: u32) -> Option<(&Dragon, usize)> {
        use horizon_client::join_seq::residency::{Source, table::EntityLookup};
        let EntityLookup::Found {
            location,
            value: Target::DragonPart(_),
        } = self.server_resident_entity(id)
        else {
            return None;
        };
        if location.owner.dimension != self.dim.key() {
            return None;
        }
        let Source::DragonPart(offset) = location.source else {
            return None;
        };
        Some((self.dragon.as_ref()?, usize::from(offset)))
    }

    pub(super) fn assign_resident_crystal_ids(&mut self) {
        super::entity_residency::assign_resident_ids!(
            self.crystals,
            c,
            &mut self.next_entity,
            None
        );
    }
}
