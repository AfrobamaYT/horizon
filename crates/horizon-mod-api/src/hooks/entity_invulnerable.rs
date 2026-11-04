//! The `entity_invulnerable` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_permanently_invulnerable"
    )]
    fn entity_permanently_invulnerable();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_set_invulnerable"
    )]
    fn entity_set_invulnerable();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_damage_cooldown_time"
    )]
    fn entity_damage_cooldown_time();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_set_damage_cooldown_time"
    )]
    fn entity_set_damage_cooldown_time();
}
