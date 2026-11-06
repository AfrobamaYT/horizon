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
    /// The slot `stack` is equipped to (`Equippable.slot`): the item's `equippable` component
    /// after the stack's own patch, which may add, change or remove it. `head`, `chest`, `legs`,
    /// `feet`, `body`, `mainhand`, `offhand` or `saddle`; none where the stack is not equippable.
    #[notes = r#"## Equipment slot

`ItemResource::equippable_slot(host)` answers which slot a stack goes in when worn
(`Equippable.slot`), from the item's `equippable` component after the stack's patch, so a
command's or a data pack's override counts. It does not say whether a given entity may wear it;
that depends on the wearer and is not answered here."#]
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_components::Game<'_>::native_item_equippable_slot"
    )]
    fn item_equippable_slot();
}
