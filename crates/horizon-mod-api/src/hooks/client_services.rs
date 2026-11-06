//! The `client_services` declarations reconciled with the fork API.

pub trait Hooks {
    /// Before an actual connection is discarded, not on a dimension change.
    #[hook(side = "client", contract = "listen")]
    fn client_disconnecting();
    /// The normal menu blur request, before finite/range validation. None disables it.
    #[hook(side = "client", contract = "listen")]
    fn menu_background();
}
