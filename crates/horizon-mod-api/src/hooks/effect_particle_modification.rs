//! The `effect_particle_modification` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `EffectParticleModificationEvent`, vanilla's `LivingEntity.updateSynchronizedMobEffectParticles`: the swirl particles of the living entity `entity`'s visible effects (a [`LivingRef`], a mob or a player) and whether they are all ambient (`ambient`, drawn fainter and rarer; true when none is visible), as the server is about to send them in its synched data (`DATA_EFFECT_PARTICLES`, `DATA_EFFECT_AMBIENCE_ID`). `particles` starts as vanilla's, one per visible effect in the order the effects are kept: an `entity_effect` in the effect's colour (alpha 38 of 255 when ambient), the omens' own particles. Add, change, reorder or clear them, and change `ambient`; an empty list shows nothing. Particles that need a block state, and item, trail and vibration particles, can't be sent and are left out. Asked when the entity's effects change (one is added, removed, or changes its amplifier, visibility or ambience), as vanilla recomputes them only then, and only while a mod listens; the answer is sent until the next change. The window's own player draws its swirls from its own effects, not from this list. Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "EffectParticleModificationEvent",
        cancellable = false
    )]
    fn effect_particle_modification();
}
