//! The `mod_state` area: host methods the game implements (`game/host/mod_state.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `mod_state` area, implemented by the game and reached through [`Host`].
pub trait ModStateHost {
    /// Side: both.
    /// The server-owned holders' attachments; `TYPE.get(host, holder)` reads
    /// one. On a client it is the copy of what the server synced: `set` and
    /// `remove` change only the copy.
    fn attachments(&mut self) -> &mut dyn AttachmentStore {
        crate::phase::unprovided(
            "Host::attachments",
            "attachments",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `mod_state` area, implemented by the game and reached through [`ServerHost`].
pub trait ModStateServerHost {
    /// Side: server.
    /// Opens a transaction, nested in the open one if any (`Transaction.open`). Only
    /// `host.transaction` reaches it, through its seal.
    fn transaction_open(&mut self, seal: TransactionSeal) {
        crate::phase::unprovided(
            "ServerHost::transaction_open",
            "transaction_open",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `native_transaction_close`, which only `host.transaction` reaches, through its seal.
    fn transaction_close(&mut self, seal: TransactionSeal, commit: bool) {
        crate::phase::unprovided(
            "ServerHost::transaction_close",
            "transaction_close",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
