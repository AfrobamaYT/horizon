# Sync 36 decisions

## Pins and scope

Integrate upstream `441f49d57e344bd56b90fd4af5c4e443ff3937c9` directly from fork main `b8af9dedf5adcb752661016d169141e4641c5c57`. Upstream's own main-push checks passed in run 43393547822. The integration includes upstream's Horizon layer and older-client protocol, graphics, upload pacing, item-use and gameplay fixes. The target is not chased after the first push.

## Conflict decisions

- Preserve the fork's generated logical game-module discovery and topic folders. Locate upstream destinations by filename, taking both halves of moves rather than retaining duplicate root modules. The new upload-pacing module belongs under `game/interface/` with the same logical module name.
- Use upstream's single non-optional `UseStart` record, carrying its captured optional stack and tick-override flag. Remove the duplicate fork fields. Preserve hook-aware start, tick, stop and release behavior, item replacement, and guest save/restore through that record; the separate remote-client field is unrelated.
- Preserve typed hook dispatch, Host ownership, lifetime-aware admission reasons, prepared item state and entity-owned random streams. The default mob loop remains borrowed; no optional parallel feature is activated.
- Keep the fork's canonical `explosion_detonate` moment rather than adding `explosion_entities`. Existing item-damage transactions implement upstream's damage-resistance behavior without a second damage path.
- Keep the maintainer-approved `MAX_CHALLENGES = 65_536` defensive limit. This sync does not introduce a new challenge-limit decision.
- Take upstream's deliberate boat-math revert (`fdd210261e`, merged by horizon-rust/horizon#15440), retaining fork mod and borrowed-owner adapters. Upstream reports that the reverted change failed its boat-fire parity harness; this session did not run that harness. Keep the corresponding minecart and mount reverts coherent.
- Preserve the richer fork collision-subject context, prediction owner updates and typed fall dispatch while incorporating powder-snow, replay-rebase and fall-sound behavior. Retain custom datapack preset cycling alongside upstream's extended Alt preset selection.
- Retain the fork's repository-owned, paginated release-metadata query and `(pull request number, merge commit)` identity check. Upstream's number-only batching solves a quota problem the fork already avoids, but cannot safely distinguish imported upstream merge subjects. Do not add a second unused batching implementation.
- Resolve locale changes by semantic three-way dictionary merge. There were no divergent same-key values; both sets of additions and intentional deletions survive. No translations were marked reviewed.
- Retain the owner's one-concern, no-size-limit fork contribution policy.

## Move restoration and verification

The integration is prepared in separate commits so the upstream merge and the restoration script, destination moves, and fork corrections can be reviewed independently. Final destination inventory, measured remnants, static-review verdict and compile-only verification are recorded in the pull request when complete. No local tests, clippy, parity harnesses or game runs are authorized for this sync.
