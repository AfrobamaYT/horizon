//! The trait mod: the launcher links this crate into the game and calls `hooks()`. Each
//! method of `Hooks` is one of Horizon's hook points; implement the ones the mod needs
//! (docs/native-mods.md, "Trait mods").
use mod_api::{self, Hooks, PlayerLoggedInEvent, ServerHost, Text};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// A player joined the server (NeoForge's `PlayerLoggedInEvent`): greet them.
    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        let line = format!("{id} says hello, {}!", event.name);
        host.tell(event.player, Text::literal(line));
    }
}
