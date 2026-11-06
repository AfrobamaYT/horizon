use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// NeoForge's `EffectParticleModificationEvent`, vanilla's
    /// `LivingEntity.updateSynchronizedMobEffectParticles`: the swirl
    /// `particles` of the living entity `entity`'s visible effects and
    /// whether they are all `ambient`, as the server is about to send them.
    /// Change, add, reorder or clear them; an empty list shows nothing.
    /// Asked when the entity's effects change, only while a mod listens; the
    /// answer is sent until the next change. Server-side.
    pub(in crate::game) fn native_effect_particle_modification(
        &self,
        entity: LivingRef,
        particles: &mut Vec<ParticleOptions>,
        ambient: &mut bool,
    ) {
        self.fire_effect_particle_modification(entity, particles, ambient);
    }
}
