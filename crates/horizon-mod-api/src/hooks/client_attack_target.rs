pub trait Hooks {
    /// Before a client's ordinary melee click attacks entities (not a spear jab), after the
    /// interaction-key cancellation hook. `targets` starts with the crosshair entity, if any.
    /// Replace, clear or append targets for redirected or multi-target hits. At most 64 entries
    /// are accepted; an oversized batch is refused as a whole. Duplicates are sent once, in
    /// order. Every target is rechecked for the current client epoch, attackability, reach,
    /// held-item range and occlusion. Clearing an entity hit does not mine the block behind
    /// it; an untouched empty list keeps vanilla's block/miss path. Server checks and cooldowns
    /// still apply to every attack. Returning `Flow::Cancel` cancels the whole batch. No NeoForge bus counterpart; client-side only.
    #[hook(
        side = "client",
        contract = "cancel",
        out = "canceled",
        cancellable = true
    )]
    fn client_attack_target();
}
