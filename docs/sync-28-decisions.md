# Sync 28 integration decisions

Target: horizon-rust/horizon at `d9b9996a4dfc61d0316daf599d7f937ccb8b95a8`.
Fork base: `5724caf147`. Common ancestor: `501446443c`.
Upstream changed 659 paths; the initial merge produced 204 conflicted paths.

## CI ownership

Keep all fork-base `.github/workflows/` and `tools/ci/` files. Upstream's runner and parity policy changes do not replace this fork's Dyna checks, static verification, runtime leases or release configuration. Retain the fork-specific parts of CI runner documentation. No CI dispatch is performed while the rollup gate holds runners.

## Launcher uninstall and identity

Take upstream's moved-data uninstall handling and its build-directory ownership guard. The default data-directory ownership check continues to use the fork's complete identity namespace rather than upstream's generic `horizon-launcher` leaf. The incoming default-path test uses the configured launcher identity so it applies to both builds. Moved locations keep upstream's `launcher`, `game` and `build` layout, and never delete the game directory.

## Developer tools and documentation

Combine upstream's optional `--harnesses` self-check flag with the fork's exact revision and dirty-worktree reporting. Preserve fork executable names in the controller harness instructions. Take upstream's new on-screen keyboard cases without changing their assertions. Layer documentation takes upstream's page form API but retains the fork dependency inventory rather than replacing it with upstream's exception count. Layer allowances preserve fork dependencies alongside incoming telemetry classification.

## Port ledger and translations

Merge port-ledger rows by Java method key: unchanged upstream rows retain the fork's relocated paths; changed upstream contracts replace the matching row, with unambiguous source paths relocated to the fork's game-area layout. Merge Thai launcher translations by key, taking upstream changes while retaining fork-only keys. Gameplay and rendering translations are resolved in their owning areas.

## Pending area integration

Gameplay, client/render and engine conflict resolutions and clean-merge audit are in progress. This document will incorporate their decisions before the first push. No compile, tests, clippy or game behavior is certified by the provisional merge.

## Engine, item and server integration

Audited all 22 assigned paths and the additional random.rs correction. Two relocated destinations are also changed: crates/horizon-server/src/lib.rs and crates/horizon-server-launch/src/config.rs. Total: 25 paths audited, 23 changed, 2 deliberately unchanged. No unresolved textual cases or requested rulings.

- Item model tests retain both the registered mod node test and upstream linear/binary threshold coverage.
- The three language files retain fork keys and take upstream added/changed translations, including Horizon UI text.
- NBT: upstream changed its old horizon-world/src/nbt.rs, now relocated to horizon-core. Ported Cursor/Key and entry/list header primitives there, preserving fork heap quotas, bounded reads, strict duplicate handling, split recursive frames, and list_fits validation. Cursor and Key must be public because structure template decoding is now in a separate crate. No obsolete world NBT file restored.
- Items retain explicit DAMAGE presence and include_damage gating. Upstream prototype-aware removal of equal DAMAGE 0 is integrated at serialization. Upstream custom durability/mining modules, tag replacement, weapon blocking, and consume parsing integrate with existing fork modules. The duplicate Items attribute_modifiers implementation is not introduced: the fork's ItemStack::default_attribute_modifiers already owns that behavior and incoming tests already call it.
- Protocol registry.rs remains unchanged from scaffold: fork registry count limits and registry-scoped tag replacement retained; upstream conflicting tag changes are documentation/parameter naming, not a missing behavior.
- Server files expose both durable and incoming chat_guard.
- Server backup security, pruning, rollback on failed restore placement, and upstream tests integrated. Interval UTF-8 and overflow corrections and tests ported to the relocated Backups::interval in server-launch/config.rs, rather than restoring duplicate backup::interval.
- Server engine retains server-launch command construction; upstream parallel_tick is forwarded by environment without duplicating launcher logic. Control line-break rejection and its incoming regression test integrated. Parallel execution itself remains game-b's single disabled gate responsibility.
- Server main.rs remains the thin library wrapper. Incoming signals::stopped cleanup is applied in relocated server/lib.rs instead. Supervisor retains launch helpers and takes upstream queued restore/update handling before every start.
- Piston events keep note_override and take upstream queue/pop deduplication routing. Existing upstream level redstone and brightness tests included.
- Structure changes take upstream streaming template decoding and saved legacy piece reads/tests. Imports use relocated horizon-core NBT/BoundingBox and retain fork block entity transform module. Saved-template int/flag use the new shared saved_legacy helpers rather than duplicate implementations.
- LegacyRandom state/gaussian conversion initializes gate to None, matching independent clone/from_state behavior. No Mob RNG API types changed here.

Verification: rustfmt edition 2024 with skip_children=true on touched Rust files; git diff --check passed. No builds, tests, clippy or pushes performed. Patch is binary-capable and based directly on scaffold. Compilation and CI remain coordinator-owned.

## Clean-merge correction and module placement

The native explosion state merge saved blast ownership twice, then restored empty duplicate values over the outer blast attribution. Remove the duplicate saves and restores, retaining the incoming player-owner flag and the fork nested-blast contract. New extinguish, frame autotuning and smithing icon modules move into their existing entities, runtime and interface areas through the committed relocation script.

## Parallel mob dispatch: authorized performance divergence

Diverges from upstream (performance only). Root authorizes serial dispatch while preserving upstream parallel implementation behind one fork-owned gate, with minimal typed adapters where required. Exact gate and adapter inventory follows the gameplay resolution. Upstream horizon-rust/horizon#15493 states “Not yet shown to give a CPU gain.” The originating horizon-rust/horizon#15056 states “This draft does not yet establish a worthwhile performance gain or acceptable fallback overhead.” No upstream measured gain is claimed here. Tests and serial behavior remain unverified locally.
