//! The `on_caught_fire` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// On an authoritative server, fire has just burned the block `state` at
    /// `pos` from its side `face`, the one that faces the fire
    /// (`FireBlock.checkBurnOut`): the cell is already fire or air, and
    /// `state` is the block as it was. It runs after the game's own action (a
    /// TNT-class block is primed, which `tnt_primed` may still cancel), and
    /// not when `block_burn` cancelled the burn. The level may be changed
    /// here, and ticks scheduled. Fire spreading has no igniting entity or
    /// item, so none is given.
    #[hook(
        side = "server",
        contract = "listen",
        scope = "block",
        key = "state",
        target = "horizon_client::game::native_hooks::on_caught_fire::GameLevel<'_>::native_on_caught_fire",
        neoforge = "IBlockExtension.onCaughtFire"
    )]
    fn on_caught_fire();
}
