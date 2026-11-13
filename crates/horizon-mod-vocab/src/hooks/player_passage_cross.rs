//! The `player_passage_cross` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `player`'s client crossed `passage` (#3775), one of the player's level's, from `from`
    /// (its feet before), and the server is about to follow: the move to `from` passed the
    /// move checks and both ends are tracked for the player, and the server has not yet
    /// worked out where the player comes out. Set `cancel` and the server refuses the
    /// crossing (`PassageCrossRefusal::Vetoed`) and puts the player back at `from`, as a
    /// portal Immersive Portals' `canTeleportEntity` turns down does. A listener that
    /// removes the passage or teleports the player refuses it too. Server-side. Diverges
    /// from upstream: new hook.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "none (NeoForge has no such event; Immersive Portals' Portal.canTeleportEntity)"
    )]
    fn player_passage_cross();
}
