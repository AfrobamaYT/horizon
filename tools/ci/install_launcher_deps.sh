#!/usr/bin/env bash
# GitHub-hosted launcher/game headers; callers retain their exact package list.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/apt_retry.sh"
# apt update normally succeeds with stale indexes after DNS failures. Treat any
# failed index as an error so installation cannot proceed until update succeeds.
retry sudo apt-get -o APT::Update::Error-Mode=any update
retry sudo apt-get install -y --no-install-recommends "$@"
