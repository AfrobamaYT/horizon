# Design: inside-block contact shapes

Provisional answer to [#1011](https://github.com/arismoko/horizon/issues/1011):
share the sweep decision, with an API-owned shape value and a restricted,
synchronous level/entity view. This is not an implemented API. No Rust changes
are proposed for immediate landing; agreement and oracle fixtures precede code.

## Vanilla and evidence

Vanilla 26.3 `Entity.checkInsideBlocks(Vec3, Vec3, collector, visited, limit)`
was inspected through `tools/rea_vanilla.py --overload-index 1 method
net.minecraft.world.entity.Entity checkInsideBlocks` from `/workspace/horizon`.
It skips air, queries `getEntityInsideCollisionShape`, evaluates block and fluid
intersection independently, then inserts the position into visited only if either
intersects. A newly visited position applies block effects first (only when block
intersection succeeds), then fluid effects (only when fluid intersection succeeds).
The canonical full-block shape has a special inclusion shortcut. Precision uses
movement distance and the deflated target box; it is not the custom shape overlap.
Preserve that distinction, iteration budgets, ordering, and movement segmentation.
A fluid-only hit must not call the block's entity_inside callback.

The issue names no NeoForge counterpart: this is a vanilla block method, not a
new event. Public [26.3 Block javadoc](https://aldak.netlify.app/javadoc/26.3.x/net/minecraft/world/level/block/Block.html)
was inaccessible during this investigation. NeoForge extensions, limits and exact
26.3 signatures remain **unverified**; require public documentation before adopting
any such contract. No NeoForge source, patches or other project's source was read.

## Current owners and proposed shared boundary

References verified at worktree HEAD `1d1de8d42b`; client paths below are relative
to `crates/horizon-client/src/`. These are current owners, not existing hook sites.

| Route | Owner | Required integration |
| --- | --- | --- |
| Player movement | `motion.rs:1697`, `motion.rs:1806`, visitation `motion.rs:1912` | Replace local shape special cases with the shared decision; preserve built-in effects |
| Mob AI contact | `mobs/ai.rs:904`, `mobs/ai.rs:3314` | Route both contact traversals through the same accepted contact trace; avoid duplicate effects |
| Arrows | `arrows.rs:1253`, `arrows.rs:1287`, visitation `arrows.rs:1321` | Preserve collector step order and independent fluid processing |
| Dropped items | `entities.rs:960` | Replace subscription-only admission with shared geometry |
| Other projectiles / game entity batches | `game/mobs/creatures/mod.rs:205` | Same sweep adapter, bound to the actual entity |
| Generic sweep | `block_contact.rs:240`, visitation `block_contact.rs:257` | Shared geometry decision before recording contacts |
| Existing server callback | `game/native_hooks/entity_inside.rs:8`; catalog `crates/horizon-mod-api/src/hooks/entity_inside.rs:3` | Retain existing notification moment; do not use it as a shape filter |

Lower geometry prepares one candidate at a time in existing traversal order.
The game adapter fires the Part query with typed HostSide, then lower geometry
applies the validated shape and returns independent block/fluid bits. A common
restricted context must work while mob/entity collections are lifted: immutable
block/fluid state reads without chunk loads, position, entity bounds and existing
lifecycle entity/level handles. Unavailable reads return an explicit unavailable
result, never synthetic air. No mutable level borrow escapes into Parts.
Integrated, LAN and dedicated simulation use ServerHost for the acting entity;
client prediction uses ClientHost and cannot perform authoritative writes.
Remote interpolation is not an authoritative contact sweep. Predict only with
matching negotiated rules and rollback-owned state; otherwise defer custom contact
effects to the server. Implementation must verify each adapter above, including
all projectile kinds, rather than assume generic coverage from its name.

## Proposed Part and shape contract

Declare `get_entity_inside_collision_shape` beside `entity_inside`, one Part
method for this moment. Query synchronously before contact admission, independently
of whether entity_inside has subscribers. Existing block ownership resolves the
provider; do not introduce listener voting or a second contact engine.
Return API-owned Wire variants `Default`, `Empty`, `FullBlock`, or `Boxes` containing
owned block-local f64 AABBs. Default uses vanilla's complete shape semantics,
including powder snow, cauldrons, end portals and tripwire. FullBlock deliberately
selects vanilla's canonical-full-block shortcut; Boxes containing the unit cube
uses geometric testing. Document this distinction rather than infer shape identity
from equivalent coordinates. The fluid shape is not replaced by this method.

Proposed mod-output budget: at most 256 boxes, finite coordinates in [-16,16],
strict min < max on each axis. Negative coordinates are valid; negative extents,
NaN, infinity and oversized arrays are invalid. These are provisional capability
limits, never limits on vanilla Default shapes or vanilla traversal capacity.
Validate length before allocation/decoding and reject the entire invalid result,
retaining Default and issuing a bounded diagnostic. Expose the negotiated budget
at registration; refuse unsupported requested capabilities with named
`UnsupportedInsideContactShapeCapability`, equally for native and wasm. Do not
silently approximate an unsupported voxel operation. Generate Wire/native/wasm
transport from the catalog; no engine VoxelShape or borrowed game pointer crosses it.

A per-state resolved provider bit makes unused dispatch O(1): no registry scan,
serialization, snapshots or extra allocation when no override exists. Existing
vanilla geometry/effect work remains. Subscription checks may skip notifications,
not fluid processing or built-in effects. Bound custom decoding and intersection
work by the advertised box count, without reducing vanilla's own capacity.

## Lifecycle, transactions and failure

Use existing lifecycle holders for entity and level identity; no second generation
counter. A candidate captures entity identity, movement segment, state/position,
and relevant world revision. Apply rechecks them in the same transaction as query
writes and refuses a stale decision, rolling back writes; never recompute after a
callback mutates the environment. Prefer a read-only query capability so shape
selection cannot mutate its own preconditions. Any permitted attachment writes
must remain rollback-owned. During lifted passes defer retirement and external
effect journal emission to root commit; no notification or packet escapes an
aborted candidate. Refuse saves during an open action savepoint through the
existing save coordinator, not a second save service.

Native unwind or wasm trap discards that invocation's output and writes, selects
Default and disables the faulty provider for the session with a bounded diagnostic.
Native abort is unrecoverable. Both targets use identical validation and recovery
semantics. Existing entity_inside notification remains at its documented batch
boundary; geometry acceptance and effect ordering are captured in the sweep trace,
not reconstructed from a later callback against a changed block.

## Acceptance matrix

Implementation requirements, not runtime results obtained for this note. Shapes
and boxes below use block-local coordinates for a stationary candidate at (0,0,0).
Run each on player, mob, arrow, item and every projectile adapter, native and wasm.

| Fixture | Expected result |
| --- | --- |
| Empty, no fluid | visited 0; block callbacks 0; fluid effects 0 |
| FullBlock, centered entity box [.25,.75] on all axes | visited 1; block callbacks 1 |
| Boxes unit cube, centered box | visited 1; block callbacks 1 |
| Boxes [0,0,0]–[1,.5,1], entity y [.6,.9] | visited 0; block callbacks 0 |
| Same shape, entity y [.25,.4] | visited 1; block callbacks 1 |
| Empty block shape, intersecting fluid | visited 1; block callbacks 0; fluid effects 1 |
| Both intersect, candidate repeated twice | visited 1; trace block then fluid, each once |
| Neither intersects on first segment, fluid intersects on second | first segment does not mark visited; second produces fluid once |
| 257 boxes, NaN, infinity, reversed or zero extents | whole result rejected; Default result equals baseline |
| Provider traps after attachment write | attachment unchanged; Default contact; provider disabled |
| Entity retired or state/revision changes before apply | stale decision refused; zero committed effects |
| Root abort during lifted pass | no retirement or journal emission; save request refused while savepoint open |
| Default / no override | exact vanilla 26.3 contact order, collector steps, precision and effects across short/long moves and special blocks |
| Override with no entity_inside listeners | geometry still controls built-in block effects; fluid unaffected |

## Proposed stages

File none for implementation at this stage. First agree representation, restricted
context and failure semantics; obtain vanilla oracle traces for special shapes,
fluid-only contact and the full-block shortcut. Then implement the shared sweep
with default-only parity across every route. Finally add the catalog Part query,
generated native/wasm transport, registration refusal, docs, sample and behavioral
fixtures together. No player-only intermediate public API should ship.

## Open questions

- Exact supported coordinate and box budget: recommend the proposed 256 / [-16,16]
  capability initially, but confirm real use cases and public documentation before
  freezing it. Preserve arbitrary vanilla defaults without truncation.
- Restricted context read surface under lifted passes: recommend immutable loaded
  block/fluid reads and entity facts only, with explicit unavailable outcomes;
  inventory required special-block reads before approving the catalog signature.
- Canonical FullBlock versus geometric unit cube: recommend explicit variants and
  oracle tests, because vanilla's shortcut makes shape identity observable.
