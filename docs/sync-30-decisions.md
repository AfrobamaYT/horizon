# Upstream sync 30 decisions

## Pin and scope

- Fork base: `ffb27f0e287c818a3b11bd66e25cb051ec82a793` (sync 29 merged).
- Upstream pin: `911aa6ddc6ec1684eaa312c7650e938789cf2193`.
- Selection evidence: newest completed-success push-to-main `.github/workflows/ci.yml` run `43183045877` at selection, completed 2026-11-12 08:59:31 UTC. Newer cancelled or unfinished runs do not qualify.
- Previous integrated upstream ancestor: `c94cdb0c76d2e06e13240eee7d9e1f807977318f`.
- Incoming delta: 21 files. Initial conflicts: 9, comprising 7 content conflicts and 2 modify/delete conflicts.

## Resolution principles

Keep typed component, equipment, entity, hook and list ownership. One-side merge wins are not evidence that behavior was integrated. Compare old incoming roots with compiled owners before removal; move tests onto the production helpers rather than reimplementing behavior in fixtures. Main CI files and the parallel mob dispatch gate remain unchanged.

## Tooling and launcher sweep

The incoming LAN port ledger now names `WorldOptions::port_tooltip` and describes invalid-port reasons while retaining the fork's relocated `game/network/lan_screen.rs` paths. The incoming launcher diagnostics distinguish a pinned version manifest without its client jar; paths are normalized for display, a wrong jar still takes precedence, and the existing missing-client case remains. Its scratch-directory regression checks both direct and Modrinth-style manifest paths and verifies no download request. Incoming diagnostic-string style is retained from upstream.

The command corpus adds TNT minecart float-field/default/clamping and stationary explosion scenarios. It remains a developer harness, not a local test executed for this sync. Its fields must correspond to the compiled minecart NBT and blast owners.

## Verification status

Whole-file upstream test inventory: 287 declarations across 13 changed test-bearing paths. Area sweeps, conflict integration and compile-only check pending. No local tests, clippy, parity harnesses or game runs.
