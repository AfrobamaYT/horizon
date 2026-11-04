//! The `item_components` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_components::Game<'_>::native_item_default_component"
    )]
    fn item_default_component();
    #[helper(
        side = "both",
        target = "horizon_client::game::native_components::Game<'_>::native_item_burn_time"
    )]
    fn item_burn_time();
    #[helper(
        side = "both",
        target = "horizon_client::game::native_components::Game<'_>::native_item_compost_chance"
    )]
    fn item_compost_chance();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_components::Game<'_>::native_item_has_enchantment_effect"
    )]
    fn item_has_enchantment_effect();
}
