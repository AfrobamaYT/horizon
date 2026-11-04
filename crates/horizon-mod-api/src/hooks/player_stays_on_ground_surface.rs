//! The `player_stays_on_ground_surface` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The player's crouch edge backoff (`Player.maybeBackOffFromEdge`) asks whether the player is staying on the ground surface (`Player.isStayingOnGroundSurface`, vanilla: the shift key is down). `entity` is the player's id; a mod may change `staying`, so a mod can make the player hold to an edge without crouching, or walk off while crouching. Fires on both sides, on the client's replayed ticks too, with the loop `context` of the side that ticks the player (its registries, settings and mod list): register the mod on both, and keep the listener a pure function of its inputs so prediction and the server agree. Beyond NeoForge: there is no counterpart. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        cancellable = false,
        version = 2,
        target = "horizon_client_lib::motion::native_player_stays_on_ground_surface"
    )]
    fn player_stays_on_ground_surface();
}
