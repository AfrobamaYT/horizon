//! The `transactions` area: host methods the game implements (`game/host/transactions.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `transactions` area, implemented by the game and reached through [`ServerHost`].
pub trait TransactionsServerHost {
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
