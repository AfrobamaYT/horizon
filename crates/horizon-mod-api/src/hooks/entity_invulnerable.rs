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

    #[notes = r#"## The entity invulnerability timer

`native_entity_invulnerable_time` and `native_entity_set_invulnerable_time` (#916) are `Entity.invulnerableTime`: the entity-level invulnerability window (`isTemporarilyInvulnerable` is it above zero), counted down by the base tick. Its own field on a mob and a player and the saved `invulnerable_time` tag on the other kinds — never a rename or alias of the living damage cooldown beside it. The permanent flag reads and writes now cover players (their body) and non-mob kinds (the saved `Invulnerable` tag the item damage path already refuses by)."#]
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_invulnerable_time"
    )]
    fn entity_invulnerable_time();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_set_invulnerable_time"
    )]
    fn entity_set_invulnerable_time();
}
