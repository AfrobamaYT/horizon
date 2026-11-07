//! The game's side of the `mod_state` area (`horizon-mod-api/src/host/mod_state.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ModStateHost for Game<'_> {
    fn attachments(&mut self) -> &mut dyn AttachmentStore {
        mod_api::__private::check_access("attachments", mod_api::__private::LoadAccess::WorldOnly);
        self.native_attachments()
    }
}
impl ModStateServerHost for Game<'_> {
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
