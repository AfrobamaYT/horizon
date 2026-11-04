//! The `transactions` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::transactions::Game<'_>::native_transaction_open"
    )]
    fn transaction_open();
    #[helper(
        side = "server",
        target = "horizon_client::game::transactions::Game<'_>::native_transaction_close"
    )]
    fn transaction_close();
}
