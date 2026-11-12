#!/usr/bin/env bash
# Keeps a Dyna slot's persistent build folder under the size at which Dyna empties it whole.
#
# Dyna measures each slot's /build before a runner starts and empties it completely above its
# cap (40 GiB), registry and toolchain included. Jobs of different kinds and branches share a
# slot and pile their artefacts up in target/. When /build ends a job above TRIM_ABOVE_GIB, this
# empties target/ only: the registry, the toolchain (also target/rustup, where wasm-mods keeps
# it) and the prepared vanilla data stay, and the compiler cache refills the build from sccache.
# A report only on failure: it never fails the job.
set -u
build=${DYNA_CI_BUILD:-}
target=${CARGO_TARGET_DIR:-}
limit_gib=${TRIM_ABOVE_GIB:-24}
if [ -z "$build" ] || [ ! -d "$build" ] || [ -z "$target" ] || [ ! -d "$target" ]; then
  echo "build-folder trim: not a Dyna slot"
  exit 0
fi
gib() { du -s --block-size=1G "$1" 2>/dev/null | cut -f1; }
before=$(gib "$build")
if [ -z "$before" ] || [ "$before" -le "$limit_gib" ]; then
  echo "build-folder trim: $build ${before:-?} GiB, at most $limit_gib GiB: kept"
  exit 0
fi
find "$target" -mindepth 1 -maxdepth 1 ! -name rustup -exec rm -rf {} + 2>/dev/null
echo "build-folder trim: $build $before GiB above $limit_gib GiB: emptied target/ (kept registry, toolchain, vanilla data), now $(gib "$build") GiB" | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"
