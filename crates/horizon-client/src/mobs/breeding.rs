//! Shared breeding inheritance and parent-stream reward decisions.
use super::{Mob, MobWorld, Species};

pub fn draws_parent_variant(species: Species) -> bool {
    matches!(
        species,
        Species::Pig
            | Species::Cow
            | Species::Chicken
            | Species::Fox
            | Species::Cat
            | Species::Wolf
    )
}

pub fn orb_after_child(species: Species) -> bool {
    species == Species::Fox
}

pub(super) fn experience(m: &mut Mob, w: &MobWorld, inherited: bool) -> super::BreedExperience {
    if inherited && !w.asks_baby_spawn && !orb_after_child(m.species()) {
        super::BreedExperience::Drawn(super::breeding_experience(m, w.mob_drops))
    } else {
        super::BreedExperience::Deferred
    }
}
