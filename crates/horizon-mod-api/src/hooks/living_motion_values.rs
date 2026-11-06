//! The `living_motion_values` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A living entity's jump power (`LivingEntity.getJumpPower`: jump strength attribute, block jump factor and Jump Boost) is about to be used by `jumpFromGround`. `entity` is its id and `kind` its type; a mod may change `power`. The rabbit's own hop and the player, cube, panda and ordinary mob jumps all pass through it. Fires on both sides, on the client's replayed ticks too, with the loop `context` of the side that ticks the entity (its registries, settings and mod list): register the mod on both, and keep the listener a pure function of its inputs so prediction and the server agree. Beyond NeoForge: there is no counterpart. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        cancellable = false,
        version = 2,
        target = "horizon_client_lib::motion::native_living_jump_power",
        neoforge = "none (vanilla LivingEntity.getJumpPower; NeoForge adds no event or extension for it); LivingEvent.LivingJumpEvent fires after, without the power"
    )]
    fn living_jump_power();
    /// A living entity's movement speed (`LivingEntity.getSpeed`) is about to drive its travel on the ground and through fluids. `entity` is its id and `kind` its type; a mod may change `speed`. Fires on both sides, on the client's replayed ticks too, with the loop `context` of the side that ticks the entity (its registries, settings and mod list): register the mod on both, and keep the listener a pure function of its inputs so prediction and the server agree. Beyond NeoForge: there is no counterpart. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        cancellable = false,
        version = 2,
        target = "horizon_client_lib::motion::native_living_speed",
        neoforge = "none (vanilla LivingEntity.getSpeed; NeoForge adds no event or extension for it)"
    )]
    fn living_speed();
    /// A living entity's airborne acceleration (`LivingEntity.getFlyingSpeed`) is about to be used by its travel. `entity` is its id and `kind` its type; a mod may change `speed`. Fires on both sides, on the client's replayed ticks too, with the loop `context` of the side that ticks the entity (its registries, settings and mod list): register the mod on both, and keep the listener a pure function of its inputs so prediction and the server agree. Beyond NeoForge: there is no counterpart. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        cancellable = false,
        version = 2,
        target = "horizon_client_lib::motion::native_living_flying_speed",
        neoforge = "none (vanilla LivingEntity.getFlyingSpeed; NeoForge adds no event or extension for it)"
    )]
    fn living_flying_speed();
}
