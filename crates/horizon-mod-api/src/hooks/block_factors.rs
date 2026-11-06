//! The `block_factors` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The jump factor of a block state this part is attached to (`Block.getJumpFactor`, which `Entity.getBlockJumpFactor` reads for the player, the cube, the panda, the rabbit and every other living jumper) is being read; vanilla's values are 0.5 for honey and 1.0 for the rest. A part may change `factor` (`event.factor`). The entity reads the block at its feet and, when that gives 1.0, the block below it, so a changed factor takes part in that choice exactly as the block's own would. It runs for each read, on both sides and on the client's replayed ticks, so register the mod on both and keep the listener a pure function of its inputs. It is given the loop `context` of the side that ticks the entity, and takes no `Host`. NeoForge's counterpart `Block.getJumpFactor()` is a method of the block, with no bus event, so this is a part method only; a mod reacting to its own blocks uses this part. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        scope = "block",
        key = "state",
        cancellable = false,
        target = "horizon_client_lib::motion::native_block_jump_factor",
        neoforge = "none (vanilla Block.getJumpFactor; NeoForge adds no event or extension for it)"
    )]
    fn block_jump_factor();
    /// The speed factor of a block state this part is attached to (`Block.getSpeedFactor`, which `Entity.getBlockSpeedFactor` reads for the player, mobs, boats and minecarts where vanilla reads it) is being read; vanilla's values are 0.4 for honey and soul sand and 1.0 for the rest. A part may change `factor` (`event.factor`). The entity reads the block at its feet and, when that is not water or a bubble column and gives 1.0, the block below it, so a changed factor takes part in that choice exactly as the block's own would. It runs for each read, on both sides and on the client's replayed ticks, so register the mod on both and keep the listener a pure function of its inputs. It is given the loop `context` of the side that ticks the entity, and takes no `Host`. NeoForge's counterpart `Block.getSpeedFactor()` is a method of the block, with no bus event, so this is a part method only; a mod reacting to its own blocks uses this part. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        scope = "block",
        key = "state",
        cancellable = false,
        target = "horizon_client_lib::motion::native_block_speed_factor",
        neoforge = "none (vanilla Block.getSpeedFactor; NeoForge adds no event or extension for it)"
    )]
    fn block_speed_factor();
}
