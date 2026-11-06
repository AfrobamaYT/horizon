#!/usr/bin/env bash
# Cargo nests RUSTC_WORKSPACE_WRAPPER (clippy-driver) inside RUSTC_WRAPPER.
# sccache cannot probe that nested compiler; keep workspace linting direct and
# cache the ordinary rustc invocations used by registry dependencies and checks.
set -euo pipefail
case "${1##*/}" in
  clippy-driver | clippy-driver.exe) exec "$@" ;;
  *)
    # Cargo has already selected incremental compilation in rustc's arguments.
    # sccache refuses CARGO_INCREMENTAL=1 even for non-incremental dependencies.
    unset CARGO_INCREMENTAL
    exec sccache "$@"
    ;;
esac
