<!-- chapter: Named hook points -->
<!-- separator: 1 -->

### Named hook points

The declarations under `crates/horizon-mod-api/src/hooks/` name each hook point and the version of its contract (signature,
when it is called, what it does): `#[hook(side = "client", contract = "listen", ...)] fn render_gui_post();`, fired by `horizon_client::game::native_hooks::Game<'_>::native_render_gui_post`.
An injection names `hook` and `hook_version` instead of `target`, and horizon-mixins resolves it from the catalog
of the source it applies to, so a hook point can move in the source without breaking the mods that use it. A name
the catalog lacks is `E_HOOK_UNKNOWN` (the error lists the names it has), a version it lacks `E_HOOK_VERSION`. A
contract is never changed, its `mod_api` entry (the trait method's contract) included: an incompatible change adds
a version. New parameters before the out-parameter, with the old ones kept in name and order, keep the version: a
binding names a parameter, and a trait mod's event gains fields; a test that calls the `dispatch_*` function passes them. Helpers (`native_text`, `native_player`…) have no hook entry; the catalog's `helpers` list names the ones
the generated `Host` offers, and they are kept stable as the hook points are. A test (`catalog_matches_hook_points`) keeps the
catalog and the hook points' sources (`native_hooks/`, `horizon-mod-render`'s `render.rs`, `horizon-world`'s `loot.rs`) in step.
Raw frame functions are native escape hatches, not methods on `Hooks`. For example, an
internals injection at `horizon_mod_render::render::world_drawn` names its listener with
`with`; use the current catalog hash and `kind = "head"`. The launcher supplies the
checked raw listener signature, and the ordinary registry owns its priority and lifetime.

#### Teleport hooks

`ender_pearl_teleport` version **2** covers player-owned and mob-owned pearls. Its `owner` is an
`EntityRef`; listeners can cancel or redirect `target`. Player owners retain the
acting-player context and mutable fall `damage`. Mob owners take no pearl damage
and make no endermite roll. Pearl placement does not emit the command-only
`entity_teleport` notification. The pearl is consumed even when teleportation is canceled.

`item_consumption_teleport` version **2** covers players and foxes with the random-teleport
consume effect. Its `consumer` is a `LivingRef`, and `consumed` is the stack captured
before use, including count and components. Cancellation ends further teleport
attempts, not consumption. Fox callbacks run before that fox's living AI/movement
and before the next mob ticks, with the full server host. Listener changes to an
actor's item, position or eligibility survive rather than being overwritten by a
stale teleport decision. Command and Ender-entity teleport hooks are unchanged.

Both version-2 contracts replace version 1's player UUID/name fields: use `owner`
or `consumer.entity()` to address the actor, and the acting-player context for
player-only operations. Named injections must request `hook_version = 2` and bind
the new parameter names; version 1 is not an alias for these contracts.
