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
    #[notes = r#"## One damage entry

`native_entity_hurt(entity, damage_type, amount, direct, causing)` is vanilla's `Entity.hurtServer(level, source, amount)`: the engine's own hurt paths take the hit — the damage type, the direct and the causing entity, armor, the damage cooldown and every hurt hook kept, and the damage event the hit already broadcasts is the broadcast. True when the damage was taken, `None` for a stale handle. `native_entity_invulnerable_to_damage` asks the same decision a hit runs (#842's guard), `native_entity_entities_team_damage_allowed` the team and friendly-fire rule, and `native_entity_invulnerable_to_piercing_weapon` the `Invulnerable` flag a piercing weapon reads."#]
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_hurt"
    )]
    fn entity_hurt();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_invulnerable_to_damage"
    )]
    fn entity_invulnerable_to_damage();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_entities_team_damage_allowed"
    )]
    fn entity_entities_team_damage_allowed();
    #[helper(
        side = "server",
        receiver = "entity",
        target = "horizon_client::game::native_entity_invulnerable::Game<'_>::native_entity_invulnerable_to_piercing_weapon"
    )]
    fn entity_invulnerable_to_piercing_weapon();
}
