//! The `mob_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_ref"
    )]
    fn mob();
    /// Current portal cooldown in ticks. Zero is a valid cooldown; `None` means
    /// the mob is stale, parked, unavailable, a client, or the separately modelled dragon.
    #[helper(
        side = "both",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_portal_cooldown"
    )]
    fn mob_portal_cooldown();

    /// The current level's mob portal eligibility decision, including riders and mounts.
    /// `Some(false)` means ineligible; `None` means a stale, parked or unavailable
    /// handle, a client, or the separately modelled dragon. Does not load a level.
    #[helper(
        side = "both",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_can_use_portal"
    )]
    fn mob_can_use_portal();
    /// The live leash holder, or `Unleashed` when no leash exists. `None` means the mob,
    /// holder or saved holder is unavailable, stale or in a parked level.
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_leash_holder"
    )]
    fn mob_leash_holder();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_target"
    )]
    fn mob_target();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_set_target"
    )]
    fn mob_set_target();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_is_aggressive"
    )]
    fn mob_is_aggressive();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_set_aggressive"
    )]
    fn mob_set_aggressive();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_persistence_required"
    )]
    fn mob_persistence_required();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_set_persistence_required"
    )]
    fn mob_set_persistence_required();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_requires_custom_persistence"
    )]
    fn mob_requires_custom_persistence();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_no_ai"
    )]
    fn mob_no_ai();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_set_no_ai"
    )]
    fn mob_set_no_ai();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_refs::Game<'_>::native_mob_set_baby"
    )]
    fn mob_set_baby();
}
