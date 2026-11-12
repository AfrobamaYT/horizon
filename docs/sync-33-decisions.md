# Upstream sync 33 decisions

## Pin and verification boundary

The sync starts at fork main `00561622057353a5e3fb83a557da68c3e2f856a3` and merges upstream `f24ee1accf1c816736e6e067939d04f3853f474e` directly. Upstream's own `ci.yml` push-to-main run [43207155875](https://github.com/horizon-rust/horizon/actions/runs/43207155875) passed. The previous synced upstream pin is `ba91f75cb21a74026319d1393c17fc5061dbd725`.

No local tests, clippy, harnesses or game runs are permitted for this sync. Only touched-file rustfmt and the full workspace/all-target compile-only check are used locally. Dependency metadata resolution does not compile or execute the new suites. Runtime claims require CI results, and resource-backed ignored assertions require the main slow shards.

## Dependency graph

- `crates/horizon-client-render/Cargo.toml`: union fork `tempfile` and `mod_platform` test dependencies with incoming `horizon-perf`; retain incoming format dependency and allocation-count feature forwarding.
- `crates/horizon-world/Cargo.toml`: retain fork `horizon-assets` and add upstream `horizon-threads`.
- `crates/horizon-server/Cargo.toml`: retain fork `horizon-bench`, add incoming direct anticheat/chat-filter dependencies.
- `crates/horizon-threads/Cargo.toml`: upstream `alloc-census = ["alloc-count"]`; retain both fork allocation-test features.
- `crates/horizon-client/Cargo.toml`: incoming allocator feature propagation and allocation counting; retain fork Windows prebuilt allocator documentation and mod test edges.
- `Cargo.lock` and `launcher/Cargo.lock`: preserve the fork's locked registry versions, resolve incoming additional packages, and keep the newly introduced protobuf packages at upstream's exact `0.14.4` versions. No pre-existing locked registry package/version is removed. Both `object` versions coexist where the profiler and fork native machinery require different versions.
- Automatic manifest merges retain the three new perf workspace members and their library/platform/official edges without removing mod dependencies.

## Workflow divergence

**Diverges from upstream: perf-wasm has no pull_request trigger, per the fork's gate policy.** Main pushes retain upstream's path filters; manual dispatch remains. The workflow uses `ubuntu-latest`, a GitHub-hosted runner, not a Dyna runner. No dispatch was made during integration. This does not add a sixth PR gate or request additional Dyna capacity.

The following exact block was removed from `.github/workflows/perf-wasm.yml`, between the push filters and `workflow_dispatch`. Restoring this block reverses the trigger divergence in one edit:

```yaml
  pull_request:
    types: [opened, synchronize, reopened]
    paths:
      - 'crates/horizon-perf/**'
      - 'crates/horizon-perf-format/**'
      - 'crates/horizon-bench/**'
      - 'xtask/**'
      - 'Cargo.lock'
      - 'Cargo.toml'
      - '.github/workflows/perf-wasm.yml'
```

## Integration inventory

The remaining per-path resolution and semantic inventory is recorded before the merge commit, after the disjoint render, gameplay, world/network/runtime and engine/tool ports are integrated. Retired gameplay source files must remain deleted; incoming behavior belongs in canonical modules, not resurrected duplicate owners.
