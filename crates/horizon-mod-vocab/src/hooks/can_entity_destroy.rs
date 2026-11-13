//! The `can_entity_destroy` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// On an authoritative server, whether `entity` may destroy the block `state` at `pos`
    /// (`IBlockExtension.canEntityDestroy`), asked where vanilla asks its per-block rule: the
    /// wither's block break (`WitherBoss.canDestroy`: not `#wither_immune`), a dangerous
    /// wither skull's blast (the entity is the skull; only a block it may destroy has its
    /// resistance capped at 0.8) and the dragon's `checkWalls` (not `#dragon_immune`; a
    /// `#dragon_transparent` block is skipped before the part is asked). `Some(true)` makes a
    /// block vanilla spares destroyable for that entity, `Some(false)` spares one it would
    /// break (the dragon is then in a wall), and `None` keeps vanilla's rule. The griefing
    /// gate (`entity_mob_griefing`) runs first, so with griefing off nothing breaks whatever
    /// the part says, and `living_destroy_block` can still keep a block the part allowed. A
    /// panicking part is contained and vanilla's rule is used. Asked every tick for each
    /// block in the dragon's head, neck and body: answer fast, from the inputs. The host is
    /// read-only. With no part attached to the state, the cost is one table lookup. Diverges
    /// from upstream: new hook.
    #[notes = r#"## Blocks an entity may destroy

- **`BlockPart::can_entity_destroy(host, event)`** (`CanEntityDestroyEvent { pos, state, entity }`): whether the entity `event.entity` may destroy the block (NeoForge's `IBlockExtension.canEntityDestroy`), asked where vanilla asks its per-block rule: the wither's block break (`WitherBoss.canDestroy`: any block but air and `#wither_immune`), a dangerous wither skull's blast, where the entity is the skull and only a block it may destroy has its explosion resistance capped at 0.8 (`WitherSkull.getBlockExplosionResistance`), and the dragon's `checkWalls` (any block but air, `#dragon_transparent` and `#dragon_immune`; a transparent block is skipped before the part is asked). `Some(true)` makes a block vanilla spares destroyable for that entity, such as obsidian for the dragon; `Some(false)` spares one it would break, and a spared block in the dragon's way puts it in a wall, as an immune one does. `None` leaves it to another part and then vanilla's rule. The order around it is vanilla's: the griefing gate first (`entity_mob_griefing`, the `mob_griefing` rule), so with griefing off nothing breaks whatever the part says, then this answer, then `living_destroy_block`, which can still keep a block the part allowed. The ravager's leaf breaking tests a block kind, not a may-destroy rule, and does not ask. A panicking part is contained with a warning and vanilla's rule is used. Authoritative servers only; the host is read-only. The dragon asks every tick for each block in its head, neck and body, so answer fast. With no part attached to the state, one table lookup per block. Diverges from upstream: new hook."#]
    #[hook(
        side = "server",
        contract = "replace",
        out = "may_destroy",
        scope = "block",
        key = "state",
        host = "read",
        target = "horizon_client::game::native_hooks::can_entity_destroy::Game<'_>::native_can_entity_destroy",
        neoforge = "IBlockExtension.canEntityDestroy"
    )]
    fn can_entity_destroy();
}
