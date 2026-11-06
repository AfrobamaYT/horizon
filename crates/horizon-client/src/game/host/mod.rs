//! The game's side of the host traits: `horizon-mod-api/src/host/<area>.rs` declares each
//! area's methods and `host/<area>.rs` here implements them, forwarding to the game's
//! `native_*` functions. `Host`, `ServerHost` and `ClientHost` are the unions of those traits, so
//! a mod reaches every area through them and the handles.
use super::Game;
use mod_api::*;
use mod_api::__private::game::*;

include!(concat!(env!("OUT_DIR"), "/game_host_modules.rs"));

impl Host for Game<'_> {
    fn backend(&self) -> Option<BackendRef<'_>> {
        Some(BackendRef::of(self))
    }
}

impl ServerHost for Game<'_> {
    fn backend_mut(&mut self) -> Option<BackendMutRef<'_>> {
        Some(BackendMutRef::of(self))
    }
}

impl ClientHost for Game<'_> {
    fn client_backend_mut(&mut self) -> Option<ClientBackendMutRef<'_>> {
        Some(ClientBackendMutRef::of(self))
    }
}

impl Backend for Game<'_> {}
impl BackendMut for Game<'_> {}
impl ClientBackendMut for Game<'_> {}
