//! The `game_rule_changed` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A server game rule `rule` (`minecraft:keep_inventory`) was set to
    /// `value` (`GameRuleChangedEvent`, from `GameRules.set` with a server:
    /// `/gamerule`, the game rules screen, the management protocol, a test
    /// environment, `server.properties`). `actor` is present when a player
    /// caused the change. The server and complete game rules are not exposed
    /// by this event (tracked on #29). Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "GameRuleChangedEvent"
    )]
    fn game_rule_changed();
}
