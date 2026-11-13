# Sync 35 decisions

## Pins and scope

- Fork base: `11fadb75ddb963c01b7bb86b9931d613f2bb48f9`.
- Upstream: `36eb98b6f6ec4f22649cbe65bbfd32bc0dc2083f`.
- Upstream's own main-push checks passed in run `43332621008`.
- Previous upstream ancestor in this base: `f24ee1accf1c816736e6e067939d04f3853f474e`.
- This directly integrates the pinned upstream into fork main, not through a stacked sync branch.

The merge began with 80 conflicts. Resolution areas were mobs/entities (23), gameplay and build wiring (24), rendering/network/saves (16), engine/tools/launcher (14), and the three lead-owned documentation/sampler/stack-dump paths. Adjacent automatic merges were audited alongside each area.

## Conflict policy

Keep fork typed ownership, attachment and join lifetimes, generated module discovery, canonical hook moments, native/wasm contracts and the launcher's repository/update route. Take upstream vanilla fixes and retain the restored Horizon layer, including telemetry, cosmetics, shop and Horizon extras. Do not revive deleted monoliths or add compatibility aliases when upstream extractions overlap existing fork modules.

### Mobs and entities

- Preserve canonical `borrowed_tick`, typed damage/loot continuations and lifetime-aware entity construction.
- Keep authoritative Equipment storage, selected-player gaze routing, full-stack piglin finish-admiring ownership and existing age/loot hook ordering.
- Port crystal/frame persistence, TNT runtime portal state, shulker cart behavior, fish voice/flop/sting behavior, goat flags, happy-ghast cloud healing and aged wither-star loot into their canonical owners.
- Initialize both fork lifetime/join state and incoming fields in `Crystal::new` and local TNT constructors.
- Keep one unconditional Vex gravity reset in `finish_tick`; preserve the unchanged endermite lifetime helper move and AI/NoAI callers.
- Move incoming fixture adaptations into the existing canonical test modules, deleting obsolete paths rather than registering duplicates.

### Gameplay and generated wiring

- Preserve generated declarations and native API generation. Add upstream staged mob-turn detection without duplicating executable resource linking.
- Keep `use_hand`, transactional menu handling and prepared-item RNG in their existing canonical implementations. Port substantive fixes from incoming extractions rather than restoring duplicate modules.
- Keep typed item-on/without-item jukebox interactions while taking component-aware song selection, loaded-clock behavior, events and statistics.
- Preserve mod dimensions and registered sea levels when taking map and world-loading fixes.
- Preserve live entity/vehicle transfer tokens. Consume `PendingEntity` in the extracted persist loader, setting TNT runtime portal context before admission/join hooks; never serialize the runtime flag into NBT.
- Extras glue remains under `game/interface/extras`, retaining counters/minimap hooks and the existing reconnect and PvP owners.
- Preserve sampler calibration and report restoration while adopting upstream's consistent dual-user helper collection.

### Rendering, protocol and saves

- Take upstream renderer submission/frustum extraction, retaining fork frustum test support and mod particle groups.
- Adapt sprite renames and first-JSON metadata parsing to open `ResourceSource` traversal rather than restoring filesystem-only loading.
- Preserve namespace-aware GUI loading and fork Mods navigation alongside shell-hosted pack settings.
- Preserve attachment initialization and typed falling-block updates while taking duplicate-id handling and projectile spawn improvements.
- Retain tool/attribute component delivery, query/cookie/sequence handling, effect metadata caching and existing packet layouts.
- Preserve biome-region/mod save modules, block-state-aware decode validation and attachments. Apply upstream block-entity position/order correction and jukebox load behavior.

### Engine, tools and launcher

- Keep split native hooks, Wire/crossing/dispatch/capability modules and the fork dependency policy; do not resurrect upstream trait-loading or injection generation paths.
- Retain fork stack-aware enchantment `select`, centralizing modifier identifier normalization in `item_data::modifier_rows` rather than reviving removed adapters.
- Keep server permission/backup behavior alongside upstream exact-label, input-validation and pruning fixes.
- Preserve launcher `REPO`, updates, pinned native toolchain, registry dependency freedom and locked game packages. Incoming junction-expression cleanup does not restore obsolete build paths.
- Keep parity tooling/corpora improvements and explicit manual-proof exclusions. No vanilla assets, prepared resources or decompiled source are added.

### Lead-owned paths

- `AGENTS.md`: retain the fork package name and document the actual extras child paths.
- `server_perf/samples.rs`: take upstream's Linux-only explanatory comment; retain the existing Linux cfg and all calibration/ratio/progress behavior.
- `horizon-threads/stack_dump.rs`: one Release readiness store inside the busy frame, Acquire readiness wait using `yield_now`. Production capture is unchanged.
- The independently merged sampler span-count fix remains unchanged.

## Maintainer-ratified query limit

Carry upstream's `MAX_CHALLENGES = 65_536` pending query-handshake address limit, including its existing-address renewal and pruning behavior. Vanilla does not impose this cap; retaining this upstream defensive limit is an explicit maintainer ruling for the sync.

## Deliberate mod-contract differences

Upstream's `explosion_entities` hook is not imported: the fork's canonical `explosion_detonate` covers that moment; entity filtering there is a follow-up (maintainer ruling). No sibling event, alias, adapter, fragment, call site or test is introduced. Unrelated explosion fixes remain. Extending canonical detonation with affected-entity read/removal requires a clean-room ordering/contract specification first.

Deferred notifications retain typed context: LivingJump owns `LoopContext`; DamagePost owns the mob snapshot, damage and full `DamageContainer`, replayed through `DamageLevel`. Default borrowed execution never enters deferral. The staged source scanner is not evidence of dynamic-mod parallel safety, and modded turns remain conservatively serialized.

The existing `upstream-parallel-mobs` Group/Islands quarantine remains at its pre-sync implementation rather than importing context-free replay into an incompatible optional path. Its existing rider/typed-level signature mismatches are static findings, not observed compiler failures. Incoming grouped notification buffering, quiet-trait-mod parallel enabling, planned-start/deferred collection and raid grouping rewrite are not imported into that quarantined path. This sync does not revive it or replace the live borrowed loop. Default workspace/all-target compilation does not claim all-features coverage.

## Function-replacement inputs and verification

Source implementation moves include renderer in-flight/frustum code, crystal construction, canonical extras children and shared mob item-stack configuration. Incoming duplicate gameplay extraction paths are rejected where the fork already owns canonical implementations. Native source patches must follow those actual owners; generated discovery remains authoritative. Client build wiring and mod-api-gen staged detection change. No point/injection mechanism or native game-rebuild fallback is introduced.

The file-size baseline is generated separately from the integrated source against the actual fork base, with the pinned upstream marker. It is never hand-edited.

Verification: compile-only workspace/all-target check is pending. No local tests, clippy, parity harnesses or game runs are permitted for this sync. Workers performed rustfmt parsing and static area audits only; CI remains the runtime proof.
