//! The `player_move` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A player's movement report is about to be applied: they would move from
    /// `from` to `to`, or turn from `from_rotation` to `to_rotation` (`[yaw,
    /// pitch]`). Heard once for each validated report whose position or
    /// rotation differs from the accepted one, after the server's own
    /// validation and before the report is committed. Set `cancel` for a real
    /// veto and not a rollback: the report is not applied and its bookkeeping
    /// is skipped, and the client is corrected to the accepted position and
    /// rotation through the existing teleport correction. A veto is never
    /// counted as an anticheat violation. The values are the exact ones: there
    /// is no once-per-tick accumulation and no threshold in the engine, so the
    /// mod filters (`if (to - from).length() < 1.0 / 16.0 { return; }` skips
    /// anything under 1/16 of a block). Their state is swapped in
    /// (`host.players().acting_player()`, `PlayerRef::tell`).
    ///
    /// The player's later reports wait in order behind a held one, which is
    /// ruled on in the same tick, and a report no mod has ruled on within 20
    /// ticks is dropped with the correction, never committed. A player's
    /// disconnect, teleport or level change drops a pending report cleanly. Not
    /// heard: a player's first move after joining, a player riding something
    /// (`vehicle_moved` hears the vehicle), and any teleport, the veto's own
    /// correction included. With no listener the report is committed in place
    /// with no extra work: nothing is held, copied or allocated. With one, the
    /// cost is one held report for each moving player a tick. Server-side.
    #[notes = r#"## Player move

- **`player_move(host, event)`**: Bukkit's `PlayerMoveEvent` for a guest's own movement, fired from `ServerGamePacketListenerImpl.handlePlayerPositionChange`'s place: after the server validated the report, before it is committed. Cancelling vetoes it; the client is sent back to the accepted position and rotation. The engine applies no threshold: a mod that wants Paper's 1/256 square block or ten degrees compares `from` and `to` itself."#]
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "none (NeoForge has no player move event; PlayerTickEvent covers the tick)",
        paper = "PlayerMoveEvent",
        cancellable = true
    )]
    fn player_move();
}
