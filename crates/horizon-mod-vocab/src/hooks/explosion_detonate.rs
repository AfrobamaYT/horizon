//! The `explosion_detonate` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0042

pub trait Hooks {
    /// An explosion at `center` in `dimension` (`minecraft:overworld`) is
    /// about to break `blocks` and reach `entities`: `ServerExplosion.explode`
    /// with its `toBlow`, before any block or entity is hit. Take out the
    /// positions to spare; they keep their block and get no fire. `entities`
    /// is `getEntities(source, box)`: every entity in the box the centre
    /// ± (2 × radius + 1), in walk order, without the exploding source, those
    /// that will ignore the blast included. Take one out and it is neither
    /// hurt nor pushed, and `explosion_knockback` is not asked for it; one a
    /// mod adds is walked as any other. Frames, paintings and other entities
    /// `LevelRef::entities_in` doesn't list can't be spared. Server-side.
    #[hook(
        side = "server",
        contract = "collect",
        out = "blocks",
        version = 2,
        neoforge = "ExplosionEvent.Detonate"
    )]
    fn explosion_detonate();
}
