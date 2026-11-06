//! The game's side of the `transactions` area (`horizon-mod-api/src/host/transactions.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl TransactionsServerHost for Game<'_> {
    fn transaction_open(&mut self, seal: TransactionSeal) {
        mod_api::__private::check_access(
            "transaction_open",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_transaction_open(seal)
    }
    fn transaction_close(&mut self, seal: TransactionSeal, commit: bool) {
        mod_api::__private::check_access(
            "transaction_close",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_transaction_close(seal, commit)
    }
}
