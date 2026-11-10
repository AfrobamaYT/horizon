# Death sources at the loot hook

The existing `loot_drops` hook receives `LootDrops::damage_source`, an optional
`HurtSource` carrying the death's damage type, causing entity ID and direct entity
ID. Non-death drops have no damage source. An environmental death still has its
damage type; causing and direct IDs are 0 when absent or unknown. A remembered
`player` is kill credit, not proof that the player caused this death.

`tool` remains the breaking or using tool for blocks and interactions. On a death
it is the killing entity's held stack captured for this death invocation, including
components, or `None` when unavailable. The snapshot is taken before the
`living_death` listeners, so even a death listener changing the killer's hand
cannot replace it. It is not a later inventory read. Each invocation owns its
snapshot; a cancelled attempt discards it, and a retry captures a fresh one.
Nested deaths cannot replace an outer invocation's snapshot.

The existing death source representation cannot recover a projectile ID: projectile
direct IDs are 0; melee direct IDs equal the causing ID. A causing mob ID does not
require that mob to remain loaded. Native and wasm listeners receive the same
borrowed view through the generated owned call transport; neither gets engine types.
