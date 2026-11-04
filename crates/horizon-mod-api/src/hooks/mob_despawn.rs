//! The `mob_despawn` hook: declaration the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `MobDespawnEvent`: the mob `mob` (of type `kind`, at `pos`, given as data because a mob taken out for its tick does not resolve through its handle yet) is about to run `Mob.checkDespawn`, which fires once per tick for every mob that is ticked or only checked for despawning, persistent ones and peaceful-mode ones included. Set `result`: `Allow` discards the mob now, `Deny` keeps it this tick, and `Default` (the starting value) leaves vanilla's rules to decide. Anything but `Default` skips vanilla's checks, including the `nextInt(800)` draw of the idle-despawn roll, so the hook fires before that draw: a mob a mod denies does not consume it. A listener is a pure function of its inputs, because the hook fires on replayed ticks. The wither and the ender dragon override `checkDespawn` without `super`, so the event is not fired for them. Server-side. Tick-time effect: one `listened` branch per mob per tick with no listeners.
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "MobDespawnEvent",
        cancellable = false
    )]
    fn mob_despawn();
}
