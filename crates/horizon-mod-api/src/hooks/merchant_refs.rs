//! The `merchant_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant"
    )]
    fn merchant();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant_offers"
    )]
    fn merchant_offers();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant_set_offer"
    )]
    fn merchant_set_offer();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant_add_offer"
    )]
    fn merchant_add_offer();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant_remove_offer"
    )]
    fn merchant_remove_offer();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_merchant_trading_player"
    )]
    fn merchant_trading_player();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_profession"
    )]
    fn merchant_profession();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_set_profession"
    )]
    fn merchant_set_profession();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_level"
    )]
    fn merchant_level();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_set_level"
    )]
    fn merchant_set_level();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_type"
    )]
    fn merchant_villager_type();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_set_type"
    )]
    fn merchant_set_villager_type();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_xp"
    )]
    fn merchant_xp();
    #[helper(
        side = "server",
        receiver = "merchant",
        target = "horizon_client::game::native_merchant_refs::Game<'_>::native_villager_set_xp"
    )]
    fn merchant_set_xp();
}
