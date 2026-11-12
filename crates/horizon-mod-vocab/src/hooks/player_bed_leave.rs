//! The `player_bed_leave` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0279

pub trait Hooks {
    /// `PlayerWakeUpEvent`: a server player is getting out of the bed at
    /// `pos` (`Player.stopSleepInBed`), heard first, while they still read
    /// as sleeping there. `wake_immediately` is vanilla's flag for skipping
    /// the wake-up fade; every Horizon caller passes false, as vanilla's
    /// morning wake and leave-bed button do. `update_level` is false when
    /// the level woke everyone at morning (`ServerLevel.wakeUpAllPlayers`)
    /// and true when the player got up: the Leave Bed button, their bed
    /// gone, a rule that no longer lets them sleep.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "PlayerWakeUpEvent",
        cancellable = false,
        version = 2
    )]
    fn player_bed_leave();
}
