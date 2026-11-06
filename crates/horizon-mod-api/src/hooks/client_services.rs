//! The `client_services` declarations reconciled with the fork API.

pub trait Hooks {
    /// Before an actual connection is discarded, not on a dimension change.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "ClientPlayerNetworkEvent.LoggingOut"
    )]
    fn client_disconnecting();
    /// The normal menu blur request, before finite/range validation. None disables it.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "none (NeoForge has no menu blur event)"
    )]
    fn menu_background();
}

pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::native_client_services::Game<'_>::native_gui_paint"
    )]
    fn gui_paint();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_client_services::Game<'_>::native_gui_nine_slice"
    )]
    fn gui_nine_slice();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_client_services::Game<'_>::native_gui_tooltip"
    )]
    fn gui_tooltip();
}
