//! The `living_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living"
    )]
    fn living();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_attribute_value"
    )]
    fn living_attribute_value();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_attribute_base"
    )]
    fn living_attribute_base();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_attributes"
    )]
    fn living_attributes();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_attribute_modifiers"
    )]
    fn living_attribute_modifiers();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_add_attribute_modifier"
    )]
    fn living_add_attribute_modifier();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_replace_attribute_modifier"
    )]
    fn living_replace_attribute_modifier();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_remove_attribute_modifier"
    )]
    fn living_remove_attribute_modifier();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_set_attribute_base"
    )]
    fn living_set_attribute_base();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_equipment"
    )]
    fn living_equipment();
    #[helper(
        side = "both",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_hand_item"
    )]
    fn living_hand_item();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_set_equipment"
    )]
    fn living_set_equipment();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_set_hand_item"
    )]
    fn living_set_hand_item();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_hurt_and_break"
    )]
    fn living_hurt_and_break();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_drop"
    )]
    fn living_drop();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_is_tamed"
    )]
    fn living_is_tamed();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_owner"
    )]
    fn living_owner();

    #[notes = r#"## The engine's own answers

Five living reads (#943) return the decision gameplay itself makes, not a copy of its conditions: `native_living_can_use_portal` the portal trip's gate (a player's: alive, nobody's passenger, not asleep; a mob's walk gate of #1275), `native_living_in_melee_range` the melee goal's own reach check (`Mob.isWithinMeleeAttackRange`), `native_living_can_freeze` the freeze tick's decision, `native_living_can_breathe_underwater` the air tick's gate, and `native_living_bed_orientation` the bed a sleeper lies in, as the engine keeps it."#]
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_can_use_portal"
    )]
    fn living_can_use_portal();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_in_melee_range"
    )]
    fn living_in_melee_range();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_can_freeze"
    )]
    fn living_can_freeze();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_can_breathe_underwater"
    )]
    fn living_can_breathe_underwater();
    #[helper(
        side = "server",
        receiver = "living",
        target = "horizon_client::game::native_living_refs::Game<'_>::native_living_bed_orientation"
    )]
    fn living_bed_orientation();
}
