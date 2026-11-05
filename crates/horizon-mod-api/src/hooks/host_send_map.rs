//! Map image delivery through a validated player handle.

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_inventories::Game<'_>::native_send_map"
    )]
    fn player_send_map();
}
