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
