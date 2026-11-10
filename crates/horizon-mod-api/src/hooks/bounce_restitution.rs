//! The `bounce_restitution` hook and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The restitution of a block state this part is attached to is being read
    /// (`IBlockExtension.getBounceRestitution`, which `Entity.getBlockBounciness` asks of the block an
    /// entity lands on): the factor its vertical speed is multiplied by when it hits the block from
    /// above. A part may change `factor` (`event.factor`); it starts at the block's own value, 1.0 for
    /// slime, 0.75 for a bed or a shelf mushroom and zero for the rest. The entity takes the larger of the factor and its
    /// own bounciness and, unless it is a living one, multiplies it by 0.8 as vanilla does, so 0.5
    /// bounces an entity half as high as slime. A sneaking player, a block tagged `#minecraft:suppresses_bounce`
    /// and a fall too slow to matter still do not bounce, and `bounce_on` still hears the bounce afterwards.
    /// Read for the player and the mobs, on both sides and on the client's replayed ticks, so register the
    /// mod on both and keep the listener a pure function of its inputs. It is given the loop `context` of
    /// the side that ticks the entity, and takes no `Host`. Dropped items keep their own slime rule for
    /// now. With no part attached to the state, the owning game checks the hook subscription and prepared
    /// API block key before any event conversion, so the cost is one table lookup. Diverges from upstream:
    /// new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        scope = "block",
        key = "state",
        cancellable = false,
        target = "horizon_client_lib::motion::native_bounce_restitution",
        neoforge = "IBlockExtension.getBounceRestitution"
    )]
    fn bounce_restitution();
}
